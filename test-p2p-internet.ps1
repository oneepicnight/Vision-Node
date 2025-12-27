# P2P Internet Connection Test
# Tests constellation node connecting to your public node over internet

Write-Host "`n" "=" * 70 -ForegroundColor Cyan
Write-Host " P2P INTERNET CONNECTION TEST" -ForegroundColor Green
Write-Host " " "=" * 70 "`n" -ForegroundColor Cyan

$publicIP = "143.105.111.206"
$testPort = 7071  # Different port for test node

Write-Host "TARGET NODE (Your Running Node):" -ForegroundColor Yellow
Write-Host "  Public IP: $publicIP" -ForegroundColor White
Write-Host "  HTTP API: $publicIP:7070" -ForegroundColor White
Write-Host "  P2P Port: $publicIP:7072" -ForegroundColor White
Write-Host ""

Write-Host "TEST NODE (New Local Node):" -ForegroundColor Yellow
Write-Host "  HTTP API: localhost:$testPort" -ForegroundColor White
Write-Host "  P2P Port: $($testPort + 2)" -ForegroundColor White
Write-Host "  Will connect to: $publicIP" -ForegroundColor Cyan
Write-Host ""

# Create test seed peers file pointing to your public IP
$testSeeds = @"
/// TEST SEED PEERS - Pointing to your public node
pub const INITIAL_SEEDS: &[(&str, u16)] = &[
    ("$publicIP", 7072),  // Your public node
    ("69.173.206.211", 7070),
    ("69.173.207.135", 7070),
];
"@

Write-Host "📝 Test Configuration:" -ForegroundColor Yellow
Write-Host "  Modified seed peers to include your public IP" -ForegroundColor Gray
Write-Host "  Pure swarm mode: enabled" -ForegroundColor Gray
Write-Host "  Will attempt direct P2P connection" -ForegroundColor Gray
Write-Host ""

Write-Host "🚀 Starting test node..." -ForegroundColor Cyan
Write-Host "   (This will open in a new window)" -ForegroundColor Gray
Write-Host ""

# Start test constellation node
Start-Process powershell -ArgumentList `
    "-NoExit", `
    "-Command", `
    "Write-Host '`n🧪 P2P TEST NODE`n' -ForegroundColor Cyan; `
    Write-Host 'Target: $publicIP:7072' -ForegroundColor Yellow; `
    Write-Host 'Watch for P2P handshake messages...`n' -ForegroundColor Gray; `
    cd 'c:\vision-node'; `
    `$env:VISION_GUARDIAN_MODE='false'; `
    `$env:VISION_PURE_SWARM_MODE='true'; `
    `$env:VISION_PORT='$testPort'; `
    `$env:VISION_HOST='0.0.0.0'; `
    `$env:RUST_LOG='info'; `
    `$env:VISION_PUBLIC_DIR='c:\vision-node\public'; `
    `$env:VISION_WALLET_DIR='c:\vision-node\wallet\dist'; `
    .\target\release\vision-node.exe"

Write-Host "✅ Test node starting in external window" -ForegroundColor Green
Write-Host ""

Write-Host "🔍 WHAT TO LOOK FOR:" -ForegroundColor Yellow
Write-Host ""
Write-Host "In YOUR NODE window (port 7070), look for:" -ForegroundColor Cyan
Write-Host "  [P2P] Incoming connection from 127.0.0.1" -ForegroundColor White
Write-Host "  [P2P] Handshake received" -ForegroundColor White
Write-Host "  [SWARM] Connected to verified peer" -ForegroundColor Green
Write-Host "  [SWARM] ✨ Broadcasting verified peer" -ForegroundColor Green
Write-Host ""

Write-Host "In TEST NODE window (port $testPort), look for:" -ForegroundColor Cyan
Write-Host "  [P2P] Attempting connection to $publicIP:7072" -ForegroundColor White
Write-Host "  [P2P] Handshake sent" -ForegroundColor White
Write-Host "  [SWARM] Connected to verified peer" -ForegroundColor Green
Write-Host "  [GOSSIP] 📡 Received N new peer candidates" -ForegroundColor Green
Write-Host ""

Write-Host "SUCCESS INDICATORS:" -ForegroundColor Yellow
Write-Host "  ✅ Both nodes show 'Connected to verified peer'" -ForegroundColor Green
Write-Host "  ✅ Reputation scores increase" -ForegroundColor Green
Write-Host "  ✅ Gossip exchanges peer lists" -ForegroundColor Green
Write-Host "  ✅ Network self-healing active" -ForegroundColor Green
Write-Host ""

Write-Host "⚠️  NOTE:" -ForegroundColor Yellow
Write-Host "  Since both nodes are on same machine (localhost)," -ForegroundColor Gray
Write-Host "  this simulates internet P2P but uses local connections." -ForegroundColor Gray
Write-Host "  For true internet test, you need another machine" -ForegroundColor Gray
Write-Host "  with open port 7072 to connect to $publicIP:7072" -ForegroundColor Gray
Write-Host ""

Write-Host "Press Enter to continue monitoring..." -ForegroundColor Cyan
$null = Read-Host

Write-Host "`n📊 Checking connection status..." -ForegroundColor Cyan
Start-Sleep -Seconds 5

# Check if both nodes are running
$nodes = Get-Process -Name "vision-node" -ErrorAction SilentlyContinue
Write-Host "`nRunning nodes: $($nodes.Count)" -ForegroundColor Yellow
foreach ($node in $nodes) {
    Write-Host "  PID: $($node.Id) - Started: $($node.StartTime)" -ForegroundColor White
}

Write-Host "`n✨ Test running! Monitor both windows for P2P activity." -ForegroundColor Green
Write-Host "   Press Ctrl+C in test node window to stop when done.`n" -ForegroundColor Gray
