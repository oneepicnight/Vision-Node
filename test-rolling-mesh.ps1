#!/usr/bin/env pwsh
# Test Rolling 1000-Peer Mesh System

Write-Host "`n=== ROLLING MESH CAPACITY TEST ===`n" -ForegroundColor Cyan

# Add Rust sled dependency for peer store manipulation
$testScript = @'
use sled::Db;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VisionPeer {
    node_id: String,
    node_tag: String,
    public_key: String,
    vision_address: String,
    ip_address: Option<String>,
    role: String,
    last_seen: i64,
    trusted: bool,
    admission_ticket_fingerprint: String,
    mood: Option<HashMap<String, serde_json::Value>>,
    health_score: i32,
    last_success: u64,
    last_failure: u64,
    fail_count: u32,
    is_seed: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = sled::open("./data/peers.db")?;
    let tree = db.open_tree("vision_peer_book")?;
    
    println!("Adding 1200 test peers to trigger eviction...");
    
    let now = chrono::Utc::now().timestamp();
    
    for i in 1..=1200 {
        let node_id = format!("test-node-{:04}", i);
        let node_tag = format!("TEST-NODE-{:04}", i);
        
        // Vary health scores to test eviction
        let health_score = match i {
            1..=100 => 80 + (i % 20) as i32,    // High health (80-100)
            101..=400 => 50 + (i % 30) as i32,   // Medium health (50-80)
            401..=800 => 20 + (i % 30) as i32,   // Low health (20-50)
            _ => 5 + (i % 15) as i32,            // Very low health (5-20)
        };
        
        // Mark some as seeds (should be protected)
        let is_seed = i <= 10;
        
        let peer = VisionPeer {
            node_id: node_id.clone(),
            node_tag: node_tag.clone(),
            public_key: format!("pubkey-{}", i),
            vision_address: format!("vision://{}@hash{}", node_tag, i),
            ip_address: Some(format!("192.168.{}.{}:7070", (i / 256) + 1, i % 256)),
            role: "constellation".to_string(),
            last_seen: now - (i as i64 * 60), // Vary last_seen
            trusted: false,
            admission_ticket_fingerprint: String::new(),
            mood: None,
            health_score,
            last_success: if health_score > 50 { now as u64 - (i as u64 * 30) } else { 0 },
            last_failure: if health_score < 50 { now as u64 - (i as u64 * 60) } else { 0 },
            fail_count: if health_score < 30 { (100 - health_score) as u32 / 10 } else { 0 },
            is_seed,
        };
        
        let json = serde_json::to_vec(&peer)?;
        tree.insert(node_id.as_bytes(), json)?;
        
        if i % 200 == 0 {
            println!("  Added {} peers...", i);
        }
    }
    
    db.flush()?;
    
    let count = tree.len();
    println!("\n✅ Added 1200 test peers to peer book");
    println!("📊 Current peer count: {}", count);
    println!("\nRestart the node to trigger capacity enforcement (MAX_PEERS=1000)");
    println!("Expected: 200 worst peers evicted, seeds protected\n");
    
    Ok(())
}
'@

# Save the Rust script
$testScript | Out-File -FilePath ".\test-add-peers.rs" -Encoding UTF8

Write-Host "📝 Created test-add-peers.rs script" -ForegroundColor Green
Write-Host "`nCompiling peer injection tool..." -ForegroundColor Yellow

# Create a minimal Cargo.toml for the test script
$cargoToml = @"
[package]
name = "test-add-peers"
version = "0.1.0"
edition = "2021"

[dependencies]
sled = "0.34"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = "0.4"
"@

New-Item -Path ".\test-add-peers-tool" -ItemType Directory -Force | Out-Null
$cargoToml | Out-File -FilePath ".\test-add-peers-tool\Cargo.toml" -Encoding UTF8
Copy-Item ".\test-add-peers.rs" ".\test-add-peers-tool\src\main.rs" -Force
New-Item -Path ".\test-add-peers-tool\src" -ItemType Directory -Force | Out-Null
Move-Item ".\test-add-peers.rs" ".\test-add-peers-tool\src\main.rs" -Force

Write-Host "Building tool..." -ForegroundColor Yellow
Push-Location ".\test-add-peers-tool"
cargo build --release 2>&1 | Select-Object -Last 5
Pop-Location

if (Test-Path ".\test-add-peers-tool\target\release\test-add-peers.exe") {
    Write-Host "`n✅ Tool compiled successfully!" -ForegroundColor Green
    
    Write-Host "`nStopping Guardian node..." -ForegroundColor Yellow
    Get-Process -Name "vision-node" -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 2
    
    Write-Host "Adding 1200 test peers..." -ForegroundColor Cyan
    & ".\test-add-peers-tool\target\release\test-add-peers.exe"
    
    Write-Host "`n=== PEER BOOK BEFORE CAPACITY ENFORCEMENT ===" -ForegroundColor Magenta
    
    # Try to read peer count from sled directly
    if (Test-Path ".\data\peers.db") {
        Write-Host "Peer database exists at: .\data\peers.db" -ForegroundColor Gray
    }
    
    Write-Host "`nRestarting Guardian to trigger eviction..." -ForegroundColor Yellow
    Start-Sleep -Seconds 2
    
    Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd 'c:\vision-node'; `$env:VISION_GUARDIAN_MODE='true'; `$env:BEACON_MODE='active'; `$env:VISION_PORT='7070'; `$env:VISION_HOST='0.0.0.0'; `$env:RUST_LOG='info'; `$env:VISION_PUBLIC_DIR='c:\vision-node\public'; `$env:VISION_WALLET_DIR='c:\vision-node\wallet\dist'; .\target\release\vision-node.exe"
    
    Write-Host "`n⏳ Waiting for Guardian to start and process eviction..." -ForegroundColor Yellow
    Start-Sleep -Seconds 10
    
    Write-Host "`n=== CHECKING STATS AFTER EVICTION ===" -ForegroundColor Cyan
    
    for ($i = 1; $i -le 3; $i++) {
        Write-Host "`nAttempt $i/3:" -ForegroundColor Gray
        $stats = curl http://localhost:7070/p2p/peers/status 2>$null | ConvertFrom-Json
        
        Write-Host "  Total peers: $($stats.total)" -ForegroundColor $(if ($stats.total -le 1000) { "Green" } else { "Red" })
        Write-Host "  Seed peers: $($stats.seeds)" -ForegroundColor Cyan
        Write-Host "  Avg health: $($stats.avg_health)" -ForegroundColor Yellow
        Write-Host "  Top peers: $($stats.top_sample.Count)" -ForegroundColor Gray
        
        if ($stats.total -le 1000) {
            Write-Host "`n✅ CAPACITY ENFORCEMENT WORKING!" -ForegroundColor Green
            Write-Host "   Evicted: $(1200 - $stats.total) peers" -ForegroundColor Yellow
            Write-Host "   Seeds protected: $($stats.seeds)" -ForegroundColor Cyan
            break
        }
        
        Start-Sleep -Seconds 5
    }
    
    Write-Host "`n=== TEST COMPLETE ===" -ForegroundColor Green
    Write-Host "Monitor the Guardian logs for:" -ForegroundColor Yellow
    Write-Host "  - '[PEER BOOK] Capacity enforcement: evicted X peers'" -ForegroundColor Gray
    Write-Host "  - '[PEER BOOK] total=X, seeds=Y, avg_health=Z' (every 5 minutes)" -ForegroundColor Gray
    
} else {
    Write-Host "`n❌ Failed to compile test tool" -ForegroundColor Red
    Write-Host "Falling back to manual test..." -ForegroundColor Yellow
    
    Write-Host "`nTo manually test:" -ForegroundColor Cyan
    Write-Host "  1. Monitor stats endpoint: curl http://localhost:7070/p2p/peers/status" -ForegroundColor Gray
    Write-Host "  2. Watch Guardian logs for 5-minute stats" -ForegroundColor Gray
    Write-Host "  3. As real peers connect, observe health scoring" -ForegroundColor Gray
}
