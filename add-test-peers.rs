// Standalone script to add 1200 test peers to Vision Node peer store
// Run with: cargo script add-test-peers.rs --dep sled=0.34 --dep serde_json=1.0

use sled::Db;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = sled::open("./data/peers.db")?;
    let tree = db.open_tree(b"vision_peer_book")?;
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    
    println!("\n🔧 Adding 1200 test peers to test rolling mesh capacity...\n");
    
    for i in 1..=1200 {
        // Vary health scores to test eviction algorithm
        let health = if i <= 100 { 
            80 + ((i % 20) as i32)        // High health: 80-100
        } else if i <= 400 { 
            50 + ((i % 30) as i32)        // Medium health: 50-80
        } else if i <= 800 { 
            20 + ((i % 30) as i32)        // Low health: 20-50
        } else { 
            5 + ((i % 15) as i32)         // Very low: 5-20
        };
        
        let peer = json!({
            "node_id": format!("test-node-{:04}", i),
            "node_tag": format!("TEST-{:04}", i),
            "public_key": format!("pubkey{}", i),
            "vision_address": format!("vision://TEST-{:04}@hash{}", i, i),
            "ip_address": format!("10.0.{}.{}:7070", (i / 256) + 1, i % 256),
            "role": "constellation",
            "last_seen": now - ((i as i64) * 60),
            "trusted": false,
            "admission_ticket_fingerprint": "",
            "mood": null,
            "health_score": health,
            "last_success": if health > 50 { (now as u64) - ((i as u64) * 30) } else { 0u64 },
            "last_failure": if health < 50 { (now as u64) - ((i as u64) * 60) } else { 0u64 },
            "fail_count": if health < 30 { ((100 - health) / 10) as u32 } else { 0u32 },
            "is_seed": i <= 10  // First 10 are seeds (protected)
        });
        
        let key = format!("test-node-{:04}", i);
        tree.insert(key.as_bytes(), serde_json::to_vec(&peer)?)?;
        
        if i % 200 == 0 {
            println!("  ✓ {} peers added...", i);
        }
    }
    
    db.flush()?;
    
    let total = tree.len();
    
    println!("\n✅ PEER INJECTION COMPLETE");
    println!("   Total peers in store: {}", total);
    println!("   Capacity limit: 1000");
    println!("   Expected eviction: {} peers", total - 1000);
    println!("\n📊 Health Distribution:");
    println!("   Seeds (protected): 10");
    println!("   High health (80-100): 100");
    println!("   Medium (50-80): 300");
    println!("   Low (20-50): 400");
    println!("   Very low (<20): 390");
    println!("\n🔄 Next: Restart the node to trigger capacity enforcement");
    println!("   Expected: Worst 200 non-seed peers evicted\n");
    
    Ok(())
}
