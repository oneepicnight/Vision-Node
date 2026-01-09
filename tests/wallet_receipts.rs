//! Integration test for wallet and receipts system
//!
//! Tests the end-to-end flow:
//! 1. Seed balance via /admin/seed-balance
//! 2. Transfer tokens via /wallet/transfer
//! 3. Verify balances updated
//! 4. Verify receipt written to /receipts/latest

use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use serde_json::json;

/// Helper to get base URL from environment or default
fn base_url() -> String {
    std::env::var("VISION_TEST_URL").unwrap_or_else(|_| "http://127.0.0.1:7070".to_string())
}

/// Helper to get admin token from environment
fn admin_token() -> String {
    std::env::var("VISION_ADMIN_TOKEN").unwrap_or_else(|_| "secret".to_string())
}

#[tokio::test]
async fn transfer_emits_receipt_and_updates_balances() {
    let client = reqwest::Client::new();
    let url = base_url();
    let token = admin_token();

    // Generate a signing key and use its verifying key as the sender address
    let mut rng = OsRng;
    let keypair = SigningKey::generate(&mut rng);
    let sender = hex::encode(keypair.verifying_key().to_bytes());
    let recipient = "b".repeat(64);

    // Step 1: Seed sender balance with 1,000,000 tokens
    println!("🌱 Seeding sender balance...");
    let seed_resp = client
        .post(&format!("{}/admin/seed-balance", url))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "address": sender,
            "amount": 1_000_000_u64
        }))
        .send()
        .await;

    match seed_resp {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            println!("   Seed response: {} - {}", status, body);

            if !status.is_success() {
                println!("   ⚠️  Seed failed - test may fail if balances not present");
            }
        }
        Err(e) => {
            println!("   ⚠️  Could not connect to node: {}", e);
            println!("   Make sure Vision Node is running on {}", url);
            panic!("Node not running - cannot execute integration test");
        }
    }

    // Step 2: Check initial sender balance
    println!("💰 Checking initial sender balance...");
    let initial_resp = client
        .get(&format!("{}/wallet/{}/balance", url, sender))
        .send()
        .await
        .expect("Failed to get sender balance");

    let initial_json: serde_json::Value = initial_resp
        .json()
        .await
        .expect("Failed to parse balance response");

    let initial_balance = initial_json["balance"]
        .as_str()
        .unwrap_or("0")
        .parse::<u64>()
        .unwrap_or(0);

    println!("   Sender initial balance: {}", initial_balance);
    assert!(
        initial_balance >= 100_000,
        "Sender needs at least 100k tokens for test"
    );

    // Step 3: Check initial recipient balance (should be 0 or low)
    println!("💰 Checking initial recipient balance...");
    let recip_initial_resp = client
        .get(&format!("{}/wallet/{}/balance", url, recipient))
        .send()
        .await
        .expect("Failed to get recipient balance");

    let recip_initial_json: serde_json::Value = recip_initial_resp
        .json()
        .await
        .expect("Failed to parse recipient balance");

    let recip_initial_balance = recip_initial_json["balance"]
        .as_str()
        .unwrap_or("0")
        .parse::<u64>()
        .unwrap_or(0);

    println!("   Recipient initial balance: {}", recip_initial_balance);

    // Step 4: Transfer 50,000 tokens from sender to recipient (signed)
    let transfer_amount = 50_000_u64;
    let fee_amount = 100_u64;

    println!(
        "📤 Transferring {} tokens (fee: {})...",
        transfer_amount, fee_amount
    );
    // Build the signed transfer request; our node expects a `nonce` and `public_key` and `signature`.
    // We use nonce=1 for a fresh sender.
    let mut transfer_body = json!({
        "from": sender,
        "to": recipient,
        "amount": transfer_amount,
        "fee": fee_amount,
        "nonce": 1u64,
        "public_key": hex::encode(keypair.verifying_key().to_bytes()),
        "signature": "", // to be filled
    });

    // Construct the canonical message that the server expects. We re-create the same bytes as
    // signable_transfer_bytes does (from, to, amount, fee, nonce, memo) so the signature verifies.
    let mut sign_msg = Vec::new();
    // from
    sign_msg.extend_from_slice(&hex::decode(&sender).unwrap());
    // to
    sign_msg.extend_from_slice(&hex::decode(&recipient).unwrap());
    // amount (u128 LE)
    sign_msg.extend_from_slice(&(transfer_amount as u128).to_le_bytes());
    // fee (u128 LE)
    sign_msg.extend_from_slice(&(fee_amount as u128).to_le_bytes());
    // nonce (u64 LE)
    sign_msg.extend_from_slice(&1u64.to_le_bytes());

    let signature = keypair.sign(&sign_msg);
    transfer_body["signature"] = serde_json::Value::String(hex::encode(signature.to_bytes()));

    let transfer_resp = client
        .post(&format!("{}/wallet/transfer", url))
        .json(&transfer_body)
        .send()
        .await
        .expect("Failed to post transfer");

    let transfer_status = transfer_resp.status();
    let transfer_json: serde_json::Value = transfer_resp
        .json()
        .await
        .expect("Failed to parse transfer response");

    println!(
        "   Transfer response: {} - {:?}",
        transfer_status, transfer_json
    );
    assert!(
        transfer_status.is_success(),
        "Transfer failed: {:?}",
        transfer_json
    );

    // Step 5: Verify sender balance decreased by (amount + fee)
    println!("✅ Verifying sender balance...");
    let sender_final_resp = client
        .get(&format!("{}/wallet/{}/balance", url, sender))
        .send()
        .await
        .expect("Failed to get updated sender balance");

    let sender_final_json: serde_json::Value = sender_final_resp
        .json()
        .await
        .expect("Failed to parse sender final balance");

    let sender_final_balance = sender_final_json["balance"]
        .as_str()
        .unwrap_or("0")
        .parse::<u64>()
        .unwrap_or(0);

    let expected_sender = initial_balance - transfer_amount - fee_amount;
    println!(
        "   Sender final balance: {} (expected: {})",
        sender_final_balance, expected_sender
    );
    assert_eq!(
        sender_final_balance, expected_sender,
        "Sender balance mismatch"
    );

    // Step 6: Verify recipient balance increased by amount
    println!("✅ Verifying recipient balance...");
    let recip_final_resp = client
        .get(&format!("{}/wallet/{}/balance", url, recipient))
        .send()
        .await
        .expect("Failed to get updated recipient balance");

    let recip_final_json: serde_json::Value = recip_final_resp
        .json()
        .await
        .expect("Failed to parse recipient final balance");

    let recip_final_balance = recip_final_json["balance"]
        .as_str()
        .unwrap_or("0")
        .parse::<u64>()
        .unwrap_or(0);

    let expected_recip = recip_initial_balance + transfer_amount;
    println!(
        "   Recipient final balance: {} (expected: {})",
        recip_final_balance, expected_recip
    );
    assert_eq!(
        recip_final_balance, expected_recip,
        "Recipient balance mismatch"
    );

    // Step 7: Verify receipt was written
    println!("📝 Checking receipts...");
    let receipts_resp = client
        .get(&format!("{}/receipts/latest?limit=5", url))
        .send()
        .await
        .expect("Failed to get receipts");

    let receipts_json: serde_json::Value = receipts_resp
        .json()
        .await
        .expect("Failed to parse receipts");

    let receipts = receipts_json["receipts"]
        .as_array()
        .expect("Expected receipts array");

    println!("   Latest {} receipts retrieved", receipts.len());

    // Find our transfer receipt
    let transfer_receipt = receipts.iter().find(|r| {
        r["kind"].as_str() == Some("transfer")
            && r["from"].as_str() == Some(sender.as_str())
            && r["to"].as_str() == Some(recipient.as_str())
    });

    assert!(
        transfer_receipt.is_some(),
        "Transfer receipt not found in latest receipts"
    );

    let receipt = transfer_receipt.unwrap();
    println!("   Found transfer receipt: {:?}", receipt);

    // Verify receipt fields
    assert_eq!(receipt["kind"].as_str(), Some("transfer"));
    assert_eq!(receipt["from"].as_str(), Some(sender.as_str()));
    assert_eq!(receipt["to"].as_str(), Some(recipient.as_str()));
    assert_eq!(
        receipt["amount"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok()),
        Some(transfer_amount)
    );
    assert_eq!(
        receipt["fee"].as_str().and_then(|s| s.parse::<u64>().ok()),
        Some(fee_amount)
    );

    println!("🎉 Test passed! Transfer emitted receipt and updated balances correctly.");
}

#[tokio::test]
async fn receipts_limit_and_ordering() {
    let client = reqwest::Client::new();
    let url = base_url();
    let token = admin_token();

    let mut rng = OsRng;
    let keypair = SigningKey::generate(&mut rng);
    let sender = hex::encode(keypair.verifying_key().to_bytes());
    let recipient = hex::encode([0x22u8; 32]);

    // Seed sufficient balance
    let _ = client
        .post(&format!("{}/admin/seed-balance", url))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "address": sender, "amount": 1000000u64 }))
        .send()
        .await
        .expect("seed balance");

    // Send 5 transfers to create receipts
    for i in 1..=5u64 {
        let mut transfer_body = json!({
            "from": sender,
            "to": recipient,
            "amount": 100u64,
            "fee": 1u64,
            "nonce": i,
            "public_key": hex::encode(keypair.verifying_key().to_bytes()),
            "signature": "",
        });
        let mut sign_msg = Vec::new();
        sign_msg.extend_from_slice(&hex::decode(&sender).unwrap());
        sign_msg.extend_from_slice(&hex::decode(&recipient).unwrap());
        sign_msg.extend_from_slice(&(100u128).to_le_bytes());
        sign_msg.extend_from_slice(&(1u128).to_le_bytes());
        sign_msg.extend_from_slice(&i.to_le_bytes());
        let signature = keypair.sign(&sign_msg);
        transfer_body["signature"] = serde_json::Value::String(hex::encode(signature.to_bytes()));

        let resp = client
            .post(&format!("{}/wallet/transfer", url))
            .json(&transfer_body)
            .send()
            .await
            .expect("transfer");
        assert!(resp.status().is_success());
    }

    // Now read latest 2 receipts
    let r2 = client
        .get(&format!("{}/receipts/latest?limit=2", url))
        .send()
        .await
        .expect("get receipts");
    assert!(r2.status().is_success());
    let j2: serde_json::Value = r2.json().await.expect("json");
    assert_eq!(j2.as_array().unwrap().len(), 2);

    // Read latest 5 receipts and assert ordering (latest-first)
    let r5 = client
        .get(&format!("{}/receipts/latest?limit=5", url))
        .send()
        .await
        .expect("get receipts");
    assert!(r5.status().is_success());
    let j5: serde_json::Value = r5.json().await.expect("json");
    let arr = j5.as_array().unwrap();
    assert_eq!(arr.len(), 5);
    // Ensure newest receipts are sorted newest-first by id
    for i in 0..arr.len() - 1 {
        let a = arr[i]["id"].as_str().unwrap();
        let b = arr[i + 1]["id"].as_str().unwrap();
        assert!(a > b, "expected a > b, got {} <= {}", a, b);
    }
}

#[tokio::test]
async fn transfer_insufficient_funds_fails() {
    let client = reqwest::Client::new();
    let url = base_url();

    // Generate a fresh signing key and use its verifying key as the sender address; no balance seeded => insufficient
    let mut rng = OsRng;
    let keypair = SigningKey::generate(&mut rng);
    let sender = hex::encode(keypair.verifying_key().to_bytes());
    let recipient = "d".repeat(64);

    println!("🧪 Testing insufficient funds scenario...");

    // Try to transfer 1 billion tokens (sender likely doesn't have this)
    // Build signed transfer (nonce=1). Because we didn't seed any balance for this sender, it should fail.
    let mut transfer_body = json!({
        "from": sender,
        "to": recipient,
        "amount": 1_000_000_000_u64,
        "fee": 100_u64,
        "nonce": 1u64,
        "public_key": hex::encode(keypair.verifying_key().to_bytes()),
        "signature": "",
    });
    // Compose message and sign
    let mut sign_msg = Vec::new();
    sign_msg.extend_from_slice(&hex::decode(&sender).unwrap());
    sign_msg.extend_from_slice(&hex::decode(&recipient).unwrap());
    sign_msg.extend_from_slice(&(1_000_000_000u128).to_le_bytes());
    sign_msg.extend_from_slice(&(100u128).to_le_bytes());
    sign_msg.extend_from_slice(&1u64.to_le_bytes());
    let signature = keypair.sign(&sign_msg);
    transfer_body["signature"] = serde_json::Value::String(hex::encode(signature.to_bytes()));

    let transfer_resp = client
        .post(&format!("{}/wallet/transfer", url))
        .json(&transfer_body)
        .send()
        .await
        .expect("Failed to post transfer");

    let status = transfer_resp.status();
    let body = transfer_resp.text().await.unwrap_or_default();

    println!("   Response: {} - {}", status, body);

    // Should fail with 4xx error
    assert!(
        status.is_client_error(),
        "Expected client error for insufficient funds"
    );
    assert!(
        body.contains("Insufficient") || body.contains("balance"),
        "Expected error message about insufficient funds"
    );

    println!("✅ Insufficient funds correctly rejected");
}

#[tokio::test]
async fn transfer_invalid_signature_fails() {
    let client = reqwest::Client::new();
    let url = base_url();

    // Generate two keypairs; sign with one and claim the other as sender (mismatch)
    let mut rng = OsRng;
    let keypair1 = SigningKey::generate(&mut rng);
    let keypair2 = SigningKey::generate(&mut rng);
    let sender = hex::encode(keypair1.verifying_key().to_bytes());
    let recipient = hex::encode(keypair2.verifying_key().to_bytes());

    // Seed the sender with enough balance so the transfer would otherwise succeed
    let token = admin_token();
    let seed_resp = client
        .post(&format!("{}/admin/seed-balance", url))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "address": sender,
            "amount": 1_000_000_u64
        }))
        .send()
        .await
        .expect("Failed to seed balance");
    assert!(seed_resp.status().is_success());

    // Now create a transfer where we sign using keypair2 but claim to be keypair1
    let mut transfer_body = json!({
        "from": sender,
        "to": recipient,
        "amount": 1000_u64,
        "fee": 10_u64,
        "nonce": 1u64,
        "public_key": hex::encode(keypair1.verifying_key().to_bytes()),
        "signature": "",
    });

    // Construct message as usual
    let mut sign_msg = Vec::new();
    sign_msg.extend_from_slice(&hex::decode(&sender).unwrap());
    sign_msg.extend_from_slice(&hex::decode(&recipient).unwrap());
    sign_msg.extend_from_slice(&(1000u128).to_le_bytes());
    sign_msg.extend_from_slice(&(10u128).to_le_bytes());
    sign_msg.extend_from_slice(&1u64.to_le_bytes());
    // BUT sign with the *wrong* keypair (keypair2) -> verification should fail
    let signature = keypair2.sign(&sign_msg);
    transfer_body["signature"] = serde_json::Value::String(hex::encode(signature.to_bytes()));

    let transfer_resp = client
        .post(&format!("{}/wallet/transfer", url))
        .json(&transfer_body)
        .send()
        .await
        .expect("Failed to post transfer");

    assert!(transfer_resp.status().is_client_error());
    let body = transfer_resp.text().await.unwrap_or_default();
    assert!(body.contains("signature") || body.contains("public_key_mismatch"));
}

#[tokio::test]
async fn transfer_invalid_address_fails() {
    let client = reqwest::Client::new();
    let url = base_url();

    println!("🧪 Testing invalid address validation...");

    // Short address (should fail)
    let invalid_sender = "short";
    let valid_recipient = "e".repeat(64);

    let transfer_resp = client
        .post(&format!("{}/wallet/transfer", url))
        .json(&json!({
            "from": invalid_sender,
            "to": valid_recipient,
            "amount": 1000_u64,
            "fee": 10_u64
        }))
        .send()
        .await
        .expect("Failed to post transfer");

    let status = transfer_resp.status();
    let body = transfer_resp.text().await.unwrap_or_default();

    println!("   Response: {} - {}", status, body);

    // Should fail with 4xx error
    assert!(
        status.is_client_error(),
        "Expected client error for invalid address"
    );
    assert!(
        body.contains("Invalid") || body.contains("address") || body.contains("64"),
        "Expected error message about invalid address format"
    );

    println!("✅ Invalid address correctly rejected");
}
