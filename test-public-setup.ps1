# Test Public Node Setup
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  Vision Node Public Setup Test" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

# Get network info
Write-Host "[1/5] Checking network configuration..." -ForegroundColor Yellow
$publicIP = (Invoke-RestMethod -Uri "https://api.ipify.org?format=json").ip
$localIP = (Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.IPAddress -notlike "127.*" -and $_.IPAddress -notlike "169.*" }).IPAddress

Write-Host "  Public IP: $publicIP" -ForegroundColor Green
Write-Host "  Local IP:  $localIP" -ForegroundColor Green
Write-Host ""

# Check if node is running
Write-Host "[2/5] Checking if Vision Node is running..." -ForegroundColor Yellow
$nodeProcess = Get-Process -Name "vision-node" -ErrorAction SilentlyContinue
if ($nodeProcess) {
    Write-Host "  ✓ Vision Node is running (PID: $($nodeProcess.Id))" -ForegroundColor Green
} else {
    Write-Host "  ✗ Vision Node is NOT running" -ForegroundColor Red
    Write-Host "    Start it first: .\vision-node.exe" -ForegroundColor Yellow
}
Write-Host ""

# Test local ports
Write-Host "[3/5] Testing local ports..." -ForegroundColor Yellow
$httpTest = Test-NetConnection -ComputerName localhost -Port 7070 -WarningAction SilentlyContinue -ErrorAction SilentlyContinue
if ($httpTest.TcpTestSucceeded) {
    Write-Host "  ✓ HTTP port 7070 is accessible" -ForegroundColor Green
} else {
    Write-Host "  ✗ HTTP port 7070 is not accessible" -ForegroundColor Red
}

$p2pTest = Test-NetConnection -ComputerName localhost -Port 7071 -WarningAction SilentlyContinue -ErrorAction SilentlyContinue  
if ($p2pTest.TcpTestSucceeded) {
    Write-Host "  ✓ P2P port 7071 is accessible" -ForegroundColor Green
} else {
    Write-Host "  ✗ P2P port 7071 is not accessible" -ForegroundColor Red
}
Write-Host ""

# Check firewall
Write-Host "[4/5] Checking Windows Firewall..." -ForegroundColor Yellow
$firewallCheck = netsh advfirewall firewall show rule name=all 2>$null | Select-String "Vision Node"
if ($firewallCheck) {
    Write-Host "  ✓ Firewall rules found" -ForegroundColor Green
} else {
    Write-Host "  ✗ No firewall rules found" -ForegroundColor Red
    Write-Host "    Run as Admin: .\setup-firewall.ps1" -ForegroundColor Yellow
}
Write-Host ""

# Check peers
Write-Host "[5/5] Checking connected peers..." -ForegroundColor Yellow
try {
    $peers = Invoke-RestMethod -Uri "http://localhost:7070/api/tcp_peers" -TimeoutSec 5 -ErrorAction Stop
    if ($peers.count -gt 0) {
        Write-Host "  ✓ Connected peers: $($peers.count)" -ForegroundColor Green
        foreach ($peer in $peers.peers) {
            Write-Host "    - $($peer.address)" -ForegroundColor Gray
        }
    } else {
        Write-Host "  ⚠ No peers connected yet" -ForegroundColor Yellow
    }
} catch {
    Write-Host "  ✗ Cannot check peers" -ForegroundColor Red
}
Write-Host ""

# Summary
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  Next Steps" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Your Public Address: $publicIP`:7071" -ForegroundColor Green
Write-Host ""
Write-Host "1. Setup Windows Firewall (as Administrator):" -ForegroundColor Yellow
Write-Host "   .\setup-firewall.ps1" -ForegroundColor White
Write-Host ""
Write-Host "2. Configure Router Port Forwarding:" -ForegroundColor Yellow
Write-Host "   External 7071 -> $localIP`:7071 (P2P)" -ForegroundColor White
Write-Host ""
Write-Host "3. Test external access:" -ForegroundColor Yellow
Write-Host "   https://www.yougetsignal.com/tools/open-ports/" -ForegroundColor White
Write-Host "   Enter: $publicIP and port 7071" -ForegroundColor White
Write-Host ""
Write-Host "4. Share with miners:" -ForegroundColor Yellow
Write-Host "   configure-peer.ps1 -PeerIP $publicIP -PeerPort 7070" -ForegroundColor White
Write-Host ""
pause
