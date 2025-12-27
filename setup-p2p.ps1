# Quick P2P Setup Script for Vision Node
# Run this as Administrator to configure firewall

param(
    [int]$Port = 7070
)

$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "ERROR: This script must be run as Administrator" -ForegroundColor Red
    Write-Host "Right-click PowerShell and select 'Run as Administrator'" -ForegroundColor Yellow
    exit 1
}

Write-Host "=== Vision Node P2P Setup ===" -ForegroundColor Cyan
Write-Host ""

# 1. Add Windows Firewall rule
Write-Host "[1] Configuring Windows Firewall..." -ForegroundColor Yellow

# Remove old rules if they exist
$oldRules = Get-NetFirewallRule -DisplayName "Vision Node*" -ErrorAction SilentlyContinue
if ($oldRules) {
    Write-Host "  Removing old firewall rules..." -ForegroundColor Gray
    $oldRules | Remove-NetFirewallRule
}

# Add new inbound rule
try {
    New-NetFirewallRule -DisplayName "Vision Node P2P (TCP $Port)" `
        -Direction Inbound `
        -Protocol TCP `
        -LocalPort $Port `
        -Action Allow `
        -Profile Any `
        -ErrorAction Stop | Out-Null
    Write-Host "  ✓ Inbound TCP rule created for port $Port" -ForegroundColor Green
} catch {
    Write-Host "  ✗ Failed to create firewall rule: $_" -ForegroundColor Red
}

# Add outbound rule (usually not needed, but doesn't hurt)
try {
    New-NetFirewallRule -DisplayName "Vision Node P2P Outbound (TCP $Port)" `
        -Direction Outbound `
        -Protocol TCP `
        -LocalPort $Port `
        -Action Allow `
        -Profile Any `
        -ErrorAction Stop | Out-Null
    Write-Host "  ✓ Outbound TCP rule created for port $Port" -ForegroundColor Green
} catch {
    Write-Host "  ✗ Failed to create outbound rule: $_" -ForegroundColor Red
}

Write-Host ""

# 2. Detect local IP
Write-Host "[2] Detecting network configuration..." -ForegroundColor Yellow
$localIPs = Get-NetIPAddress -AddressFamily IPv4 | Where-Object { 
    $_.IPAddress -notlike "127.*" -and 
    $_.IPAddress -notlike "169.*" -and
    $_.AddressState -eq "Preferred"
}

if ($localIPs.Count -gt 0) {
    Write-Host "  Your local IP address(es):" -ForegroundColor Gray
    foreach ($ip in $localIPs) {
        Write-Host "    - $($ip.IPAddress) ($($ip.InterfaceAlias))" -ForegroundColor White
    }
    $primaryIP = $localIPs[0].IPAddress
} else {
    Write-Host "  ✗ Could not detect local IP" -ForegroundColor Red
    $primaryIP = "192.168.1.X"
}
Write-Host ""

# 3. Detect public IP
Write-Host "[3] Detecting public IP address..." -ForegroundColor Yellow
try {
    $publicIP = (Invoke-RestMethod -Uri "https://api.ipify.org" -TimeoutSec 10).Trim()
    Write-Host "  ✓ Your public IP: $publicIP" -ForegroundColor Green
} catch {
    Write-Host "  ✗ Could not detect public IP" -ForegroundColor Red
    $publicIP = "YOUR.PUBLIC.IP.HERE"
}
Write-Host ""

# 4. Router configuration instructions
Write-Host "[4] Router Port Forwarding Configuration" -ForegroundColor Yellow
Write-Host ""
Write-Host "  To allow external connections, configure your router:" -ForegroundColor White
Write-Host ""
Write-Host "  Step 1: Find your router's IP" -ForegroundColor Cyan
Write-Host "    Usually: 192.168.0.1 or 192.168.1.1" -ForegroundColor Gray
$gateway = (Get-NetRoute -DestinationPrefix "0.0.0.0/0" | Select-Object -First 1).NextHop
if ($gateway) {
    Write-Host "    Your router IP: $gateway" -ForegroundColor White
}
Write-Host ""
Write-Host "  Step 2: Log into router admin panel" -ForegroundColor Cyan
Write-Host "    Open browser: http://$gateway" -ForegroundColor Gray
Write-Host "    (Default credentials are often on router label)" -ForegroundColor Gray
Write-Host ""
Write-Host "  Step 3: Find Port Forwarding settings" -ForegroundColor Cyan
Write-Host "    Look for:" -ForegroundColor Gray
Write-Host "      - Port Forwarding" -ForegroundColor Gray
Write-Host "      - Virtual Server" -ForegroundColor Gray
Write-Host "      - NAT Forwarding" -ForegroundColor Gray
Write-Host "      - Applications & Gaming" -ForegroundColor Gray
Write-Host ""
Write-Host "  Step 4: Add port forwarding rule" -ForegroundColor Cyan
Write-Host "    Service Name:    Vision Node P2P" -ForegroundColor Gray
Write-Host "    External Port:   $Port" -ForegroundColor White
Write-Host "    Internal IP:     $primaryIP" -ForegroundColor White
Write-Host "    Internal Port:   $Port" -ForegroundColor White
Write-Host "    Protocol:        TCP (or TCP+UDP)" -ForegroundColor Gray
Write-Host ""

# 5. UPnP alternative
Write-Host "[5] Alternative: UPnP (if supported by router)" -ForegroundColor Yellow
Write-Host "  Some routers support automatic port forwarding via UPnP" -ForegroundColor Gray
Write-Host "  Check your router admin panel to enable UPnP/NAT-PMP" -ForegroundColor Gray
Write-Host ""

# 6. Testing instructions
Write-Host "[6] Testing Your Configuration" -ForegroundColor Yellow
Write-Host ""
Write-Host "  After configuring port forwarding:" -ForegroundColor White
Write-Host ""
Write-Host "  1. Start your Vision node:" -ForegroundColor Cyan
Write-Host "     cd C:\vision-node" -ForegroundColor Gray
Write-Host "     cargo run --release" -ForegroundColor Gray
Write-Host ""
Write-Host "  2. Run connectivity test:" -ForegroundColor Cyan
Write-Host "     .\test-p2p-connectivity.ps1 -PublicIP $publicIP" -ForegroundColor Gray
Write-Host ""
Write-Host "  3. Test from external site:" -ForegroundColor Cyan
Write-Host "     https://www.yougetsignal.com/tools/open-ports/" -ForegroundColor Gray
Write-Host "     Enter IP: $publicIP" -ForegroundColor White
Write-Host "     Enter Port: $Port" -ForegroundColor White
Write-Host ""

# 7. Your P2P address
Write-Host "[7] Share This Address With Others" -ForegroundColor Yellow
Write-Host ""
Write-Host "  Your Vision Node P2P Address:" -ForegroundColor White
Write-Host "    http://$publicIP`:$Port" -ForegroundColor Green
Write-Host ""
Write-Host "  Others can connect with:" -ForegroundColor Gray
Write-Host "    curl -X POST http://their-node:7070/peer/add?token=ADMIN_TOKEN \\" -ForegroundColor Gray
Write-Host "      -H 'Content-Type: application/json' \\" -ForegroundColor Gray
Write-Host "      -d '{\"url\": \"http://$publicIP`:$Port\"}'" -ForegroundColor Gray
Write-Host ""

# 8. ngrok alternative
Write-Host "[8] Alternative: ngrok (No Router Config Needed)" -ForegroundColor Yellow
Write-Host ""
Write-Host "  If you can't configure port forwarding:" -ForegroundColor White
Write-Host ""
Write-Host "  1. Download ngrok: https://ngrok.com/download" -ForegroundColor Gray
Write-Host "  2. Run: ngrok http $Port" -ForegroundColor Gray
Write-Host "  3. Use the provided URL (e.g., https://abc123.ngrok.io)" -ForegroundColor Gray
Write-Host "  4. Share that URL as your P2P address" -ForegroundColor Gray
Write-Host ""

# 9. Environment variable check
Write-Host "[9] Environment Variables" -ForegroundColor Yellow
$visionPort = $env:VISION_PORT
$adminToken = $env:VISION_ADMIN_TOKEN

if ([string]::IsNullOrEmpty($visionPort)) {
    Write-Host "  ⚠ VISION_PORT not set (will default to 7070)" -ForegroundColor Yellow
    Write-Host "    Set it: `$env:VISION_PORT = $Port" -ForegroundColor Gray
} else {
    Write-Host "  ✓ VISION_PORT = $visionPort" -ForegroundColor Green
}

if ([string]::IsNullOrEmpty($adminToken)) {
    Write-Host "  ⚠ VISION_ADMIN_TOKEN not set (required for admin operations)" -ForegroundColor Yellow
    Write-Host "    Set it: `$env:VISION_ADMIN_TOKEN = 'your-secret-token'" -ForegroundColor Gray
} else {
    Write-Host "  ✓ VISION_ADMIN_TOKEN is set" -ForegroundColor Green
}
Write-Host ""

Write-Host "=== Setup Complete ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps:" -ForegroundColor White
Write-Host "  1. Configure router port forwarding (see instructions above)" -ForegroundColor Gray
Write-Host "  2. Start Vision node: cargo run --release" -ForegroundColor Gray
Write-Host "  3. Test connectivity: .\test-p2p-connectivity.ps1" -ForegroundColor Gray
Write-Host ""
