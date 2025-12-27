// pow_message_bytes_test.rs
// Test that pow_message_bytes produces identical output regardless of "0x" prefix

#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub parent_hash: String,
    pub number: u64,
    pub timestamp: u64,
    pub difficulty: u64,
    pub nonce: u64,
    pub pow_hash: String,
    pub state_root: String,
    pub tx_root: String,
    pub receipts_root: String,
    pub da_commitment: Option<String>,
    pub base_fee_per_gas: u128,
}

#[inline]
fn normalize_hash(s: &str) -> String {
    let trimmed = if s.starts_with("0x") || s.starts_with("0X") {
        &s[2..]
    } else {
        s
    };
    trimmed.to_lowercase()
}

fn pow_message_bytes(h: &BlockHeader) -> Vec<u8> {
    let mut msg = Vec::with_capacity(256);
    
    let parent_norm = normalize_hash(&h.parent_hash);
    let parent_bytes = parent_norm.as_bytes();
    msg.extend_from_slice(&(parent_bytes.len() as u32).to_be_bytes());
    msg.extend_from_slice(parent_bytes);
    
    msg.extend_from_slice(&h.number.to_be_bytes());
    msg.extend_from_slice(&h.timestamp.to_be_bytes());
    msg.extend_from_slice(&h.difficulty.to_be_bytes());
    
    let state_norm = normalize_hash(&h.state_root);
    let state_root_bytes = state_norm.as_bytes();
    msg.extend_from_slice(&(state_root_bytes.len() as u32).to_be_bytes());
    msg.extend_from_slice(state_root_bytes);
    
    let tx_norm = normalize_hash(&h.tx_root);
    let tx_root_bytes = tx_norm.as_bytes();
    msg.extend_from_slice(&(tx_root_bytes.len() as u32).to_be_bytes());
    msg.extend_from_slice(tx_root_bytes);
    
    let receipts_norm = normalize_hash(&h.receipts_root);
    let receipts_root_bytes = receipts_norm.as_bytes();
    msg.extend_from_slice(&(receipts_root_bytes.len() as u32).to_be_bytes());
    msg.extend_from_slice(receipts_root_bytes);
    
    if let Some(ref da) = h.da_commitment {
        let da_norm = normalize_hash(da);
        let da_bytes = da_norm.as_bytes();
        msg.extend_from_slice(&(da_bytes.len() as u32).to_be_bytes());
        msg.extend_from_slice(da_bytes);
    }
    
    msg.extend_from_slice(&h.base_fee_per_gas.to_be_bytes());
    
    msg
}

fn main() {
    println!("=== pow_message_bytes Cross-Platform Stability Test ===\n");
    
    // Test 1: Same block with and without "0x" prefix
    let h1 = BlockHeader {
        parent_hash: "abc123".to_string(),
        number: 100,
        timestamp: 1700000000,
        difficulty: 20,
        nonce: 12345,
        pow_hash: "def456".to_string(),
        state_root: "111111".to_string(),
        tx_root: "222222".to_string(),
        receipts_root: "333333".to_string(),
        da_commitment: None,
        base_fee_per_gas: 1000000000,
    };
    
    let h2 = BlockHeader {
        parent_hash: "0xabc123".to_string(),
        number: 100,
        timestamp: 1700000000,
        difficulty: 20,
        nonce: 12345,
        pow_hash: "0xdef456".to_string(),
        state_root: "0x111111".to_string(),
        tx_root: "0x222222".to_string(),
        receipts_root: "0x333333".to_string(),
        da_commitment: None,
        base_fee_per_gas: 1000000000,
    };
    
    let h3 = BlockHeader {
        parent_hash: "0xABC123".to_string(),
        number: 100,
        timestamp: 1700000000,
        difficulty: 20,
        nonce: 12345,
        pow_hash: "0xDEF456".to_string(),
        state_root: "0X111111".to_string(),
        tx_root: "0X222222".to_string(),
        receipts_root: "0X333333".to_string(),
        da_commitment: None,
        base_fee_per_gas: 1000000000,
    };
    
    let msg1 = pow_message_bytes(&h1);
    let msg2 = pow_message_bytes(&h2);
    let msg3 = pow_message_bytes(&h3);
    
    println!("Test 1: Hash prefix variations");
    println!("  No prefix:    {} bytes", msg1.len());
    println!("  With '0x':    {} bytes", msg2.len());
    println!("  Uppercase:    {} bytes", msg3.len());
    
    if msg1 == msg2 && msg2 == msg3 {
        println!("  ✅ PASS: All three encodings are identical\n");
    } else {
        println!("  ❌ FAIL: Encodings differ!\n");
        println!("    msg1: {:?}", msg1);
        println!("    msg2: {:?}", msg2);
        println!("    msg3: {:?}", msg3);
        std::process::exit(1);
    }
    
    // Test 2: Verify parent_hash field encoding
    println!("Test 2: Parent hash encoding breakdown");
    println!("  h1.parent_hash = {:?}", h1.parent_hash);
    println!("  h2.parent_hash = {:?}", h2.parent_hash);
    println!("  h3.parent_hash = {:?}", h3.parent_hash);
    
    let norm1 = normalize_hash(&h1.parent_hash);
    let norm2 = normalize_hash(&h2.parent_hash);
    let norm3 = normalize_hash(&h3.parent_hash);
    
    println!("  After normalize:");
    println!("    norm1 = {:?}", norm1);
    println!("    norm2 = {:?}", norm2);
    println!("    norm3 = {:?}", norm3);
    
    if norm1 == norm2 && norm2 == norm3 {
        println!("  ✅ PASS: All normalize to same value\n");
    } else {
        println!("  ❌ FAIL: Normalization failed\n");
        std::process::exit(1);
    }
    
    // Test 3: Verify numeric field encoding (big-endian)
    println!("Test 3: Numeric field endianness");
    let test_num: u64 = 0x0102030405060708;
    let be_bytes = test_num.to_be_bytes();
    println!("  u64 0x{:016x} -> BE bytes: {:02x?}", test_num, be_bytes);
    if be_bytes[0] == 0x01 && be_bytes[7] == 0x08 {
        println!("  ✅ PASS: Big-endian encoding correct\n");
    } else {
        println!("  ❌ FAIL: Endianness wrong\n");
        std::process::exit(1);
    }
    
    // Test 4: Empty da_commitment handling
    println!("Test 4: Optional field handling");
    let h_no_da = BlockHeader {
        parent_hash: "abc".to_string(),
        number: 1,
        timestamp: 1,
        difficulty: 1,
        nonce: 1,
        pow_hash: "def".to_string(),
        state_root: "aaa".to_string(),
        tx_root: "bbb".to_string(),
        receipts_root: "ccc".to_string(),
        da_commitment: None,
        base_fee_per_gas: 1,
    };
    
    let h_with_da = BlockHeader {
        parent_hash: "abc".to_string(),
        number: 1,
        timestamp: 1,
        difficulty: 1,
        nonce: 1,
        pow_hash: "def".to_string(),
        state_root: "aaa".to_string(),
        tx_root: "bbb".to_string(),
        receipts_root: "ccc".to_string(),
        da_commitment: Some("0xda123".to_string()),
        base_fee_per_gas: 1,
    };
    
    let msg_no_da = pow_message_bytes(&h_no_da);
    let msg_with_da = pow_message_bytes(&h_with_da);
    
    println!("  No da_commitment:   {} bytes", msg_no_da.len());
    println!("  With da_commitment: {} bytes", msg_with_da.len());
    
    if msg_no_da.len() < msg_with_da.len() {
        println!("  ✅ PASS: da_commitment changes message length correctly\n");
    } else {
        println!("  ❌ FAIL: da_commitment not encoded properly\n");
        std::process::exit(1);
    }
    
    println!("=== ALL TESTS PASSED ===");
    println!("\nThis test vector can be run on Windows and Linux to verify");
    println!("that pow_message_bytes produces identical output across platforms.");
}
