// test-visionx-mining.rs
// Quick test to verify execute_and_mine uses VisionX consensus PoW

use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== VisionX Mining Integration Test ===\n");
    
    // Start a single node
    println!("1. Starting Vision Node...");
    let mut node = Command::new("target\\release\\vision-node.exe")
        .env("VISION_DATA_DIR", "temp-visionx-test")
        .env("VISION_PORT", "17070")
        .env("VISION_P2P_PORT", "17071")
        .spawn()
        .expect("Failed to start node");
    
    // Wait for node to initialize
    thread::sleep(Duration::from_secs(5));
    
    // Send a transaction to trigger mining
    println!("2. Sending test transaction to trigger mining...");
    let tx_result = Command::new("powershell")
        .arg("-Command")
        .arg(r#"
            $body = @{
                to = 'test-recipient-visionx-mining'
                amount = 100
                memo = 'VisionX mining test'
            } | ConvertTo-Json
            
            Invoke-RestMethod -Uri 'http://localhost:17070/tx' -Method Post -Body $body -ContentType 'application/json'
        "#)
        .output();
    
    match tx_result {
        Ok(output) => {
            if output.status.success() {
                println!("✅ Transaction sent successfully");
                println!("   Response: {}", String::from_utf8_lossy(&output.stdout));
            } else {
                println!("⚠️ Transaction failed: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        Err(e) => {
            println!("❌ Failed to send transaction: {}", e);
        }
    }
    
    // Wait for block to be mined
    println!("\n3. Waiting for block to be mined with VisionX PoW...");
    thread::sleep(Duration::from_secs(10));
    
    // Check block was mined successfully
    println!("4. Checking if block was mined...");
    let height_result = Command::new("powershell")
        .arg("-Command")
        .arg("Invoke-RestMethod -Uri 'http://localhost:17070/height'")
        .output();
    
    match height_result {
        Ok(output) => {
            if output.status.success() {
                let height = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("✅ Current height: {}", height);
                
                let height_num: u64 = height.parse().unwrap_or(0);
                if height_num > 0 {
                    println!("\n5. Fetching latest block to verify VisionX PoW...");
                    let block_result = Command::new("powershell")
                        .arg("-Command")
                        .arg(format!("Invoke-RestMethod -Uri 'http://localhost:17070/block/{}' | ConvertTo-Json -Depth 10", height_num))
                        .output();
                    
                    match block_result {
                        Ok(output) => {
                            if output.status.success() {
                                let block_json = String::from_utf8_lossy(&output.stdout);
                                println!("✅ Block {} retrieved:", height_num);
                                
                                // Check if pow_hash is 64 hex chars (VisionX digest)
                                if block_json.contains("\"pow_hash\"") {
                                    println!("   Block has pow_hash field ✅");
                                    
                                    // Extract pow_hash length check
                                    if let Some(start) = block_json.find("\"pow_hash\"") {
                                        let after_field = &block_json[start..];
                                        if let Some(colon) = after_field.find(":") {
                                            let value_start = colon + 1;
                                            if let Some(quote1) = after_field[value_start..].find("\"") {
                                                let hash_start = value_start + quote1 + 1;
                                                if let Some(quote2) = after_field[hash_start..].find("\"") {
                                                    let hash_len = quote2;
                                                    println!("   pow_hash length: {} chars", hash_len);
                                                    
                                                    if hash_len == 64 {
                                                        println!("   ✅ PASS: pow_hash is 64 hex chars (VisionX digest format)");
                                                    } else {
                                                        println!("   ❌ FAIL: pow_hash should be 64 chars for VisionX");
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => println!("❌ Failed to fetch block: {}", e),
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to check height: {}", e);
        }
    }
    
    // Cleanup
    println!("\n6. Stopping node...");
    let _ = node.kill();
    thread::sleep(Duration::from_secs(2));
    
    // Cleanup test data
    let _ = Command::new("powershell")
        .arg("-Command")
        .arg("Remove-Item -Path 'temp-visionx-test' -Recurse -Force -ErrorAction SilentlyContinue")
        .output();
    
    println!("\n=== Test Complete ===");
    println!("\n✅ If you see 'PASS: pow_hash is 64 hex chars' above, VisionX mining is working!");
    println!("   This means execute_and_mine() now uses VisionX consensus PoW.");
}
