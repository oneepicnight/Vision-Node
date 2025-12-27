#!/usr/bin/env python3
"""
Add 1200 test peers to Vision Node peer store to test rolling mesh capacity enforcement.
Requires: pip install sled-py (or use Rust script instead)
"""

import json
import time
import os
import sys

# Since Python sled bindings are limited, create a simple JSON approach
# We'll create a Rust one-liner instead

rust_code = '''
use sled::Db;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = sled::open("./data/peers.db")?;
    let tree = db.open_tree(b"vision_peer_book")?;
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    
    println!("Adding 1200 test peers...");
    
    for i in 1..=1200 {
        let health = if i <= 100 { 80 + (i % 20) } 
                    else if i <= 400 { 50 + (i % 30) }
                    else if i <= 800 { 20 + (i % 30) }
                    else { 5 + (i % 15) };
        
        let peer = json!({
            "node_id": format!("test-node-{:04}", i),
            "node_tag": format!("TEST-{:04}", i),
            "public_key": format!("pk{}", i),
            "vision_address": format!("vision://TEST-{:04}@hash{}", i, i),
            "ip_address": format!("10.0.{}.{}:7070", i / 256, i % 256),
            "role": "constellation",
            "last_seen": now - (i as i64 * 60),
            "trusted": false,
            "admission_ticket_fingerprint": "",
            "mood": null,
            "health_score": health as i32,
            "last_success": if health > 50 { (now as u64) - (i * 30) } else { 0u64 },
            "last_failure": if health < 50 { (now as u64) - (i * 60) } else { 0u64 },
            "fail_count": if health < 30 { (100 - health) / 10 } else { 0u32 },
            "is_seed": i <= 10
        });
        
        let key = format!("test-node-{:04}", i);
        tree.insert(key.as_bytes(), serde_json::to_vec(&peer)?)?;
        
        if i % 200 == 0 {
            println!("  {} peers added...", i);
        }
    }
    
    db.flush()?;
    println!("\\n✓ Added 1200 test peers (capacity: 1000)");
    println!("  Seeds: 10 (protected from eviction)");
    println!("  High health (80-100): 100 peers");
    println!("  Medium health (50-80): 300 peers");
    println!("  Low health (20-50): 400 peers");
    println!("  Very low (<20): 390 peers");
    println!("\\nRestart node to trigger eviction of 200 worst peers.");
    
    Ok(())
}
'''

print(rust_code)
print("\n" + "="*60)
print("Save the above Rust code and compile with:")
print("  cargo script <filename.rs> --dep sled --dep serde_json")
print("="*60)
