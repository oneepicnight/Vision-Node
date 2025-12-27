# Vision Node P2P Connectivity Diagnostic Tool
# Run this to test if your node is accessible publicly

param(
    [string]$NodeUrl = "http://127.0.0.1:7070",
    [string]$PublicIP = "",
    [int]$Port = 7070
)

Write-Host "=== Vision Node P2P Connectivity Diagnostic ===" -ForegroundColor Cyan
Write-Host ""

# 1. Check if node is running locally
Write-Host "[1] Checking if node is running locally..." -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$NodeUrl/info" -Method Get -TimeoutSec 5
    Write-Host "✓ Node is running locally" -ForegroundColor Green
    Write-Host "  Version: $($response.version)" -ForegroundColor Gray
} catch {
    Write-Host "✗ Node is NOT running locally at $NodeUrl" -ForegroundColor Red
    Write-Host "  Start the node first: cargo run --release" -ForegroundColor Yellow
    exit 1
}
Write-Host ""

# 2. Check current peers
Write-Host "[2] Checking current peer connections..." -ForegroundColor Yellow
try {
    $peers = Invoke-RestMethod -Uri "$NodeUrl/peers" -Method Get
    Write-Host "  Current peers: $($peers.peers.Count)" -ForegroundColor Gray
    if ($peers.peers.Count -gt 0) {
        foreach ($peer in $peers.peers) {
            Write-Host "    - $peer" -ForegroundColor Gray
        }
    } else {
        Write-Host "    (no peers connected)" -ForegroundColor Gray
    }
} catch {
    Write-Host "✗ Failed to get peers: $_" -ForegroundColor Red
}
Write-Host ""

# 3. Detect public IP
Write-Host "[3] Detecting your public IP address..." -ForegroundColor Yellow
if ([string]::IsNullOrEmpty($PublicIP)) {
    try {
        $PublicIP = (Invoke-RestMethod -Uri "https://api.ipify.org" -TimeoutSec 10).Trim()
        Write-Host "✓ Detected public IP: $PublicIP" -ForegroundColor Green
    } catch {
        Write-Host "✗ Failed to detect public IP" -ForegroundColor Red
        Write-Host "  Manually specify: -PublicIP <your-ip>" -ForegroundColor Yellow
    }
}
Write-Host ""

# 4. Check if port is open locally
Write-Host "[4] Checking if port $Port is listening..." -ForegroundColor Yellow
$listening = Get-NetTCPConnection -State Listen -LocalPort $Port -ErrorAction SilentlyContinue
if ($listening) {
    Write-Host "✓ Port $Port is listening locally" -ForegroundColor Green
    Write-Host "  Process: $($listening.OwningProcess)" -ForegroundColor Gray
} else {
    Write-Host "✗ Port $Port is NOT listening" -ForegroundColor Red
    Write-Host "  Make sure VISION_PORT=$Port environment variable is set" -ForegroundColor Yellow
}
Write-Host ""

# 5. Check Windows Firewall
Write-Host "[5] Checking Windows Firewall rules..." -ForegroundColor Yellow
$fwRule = Get-NetFirewallRule -DisplayName "Vision Node*" -ErrorAction SilentlyContinue
if ($fwRule) {
    Write-Host "✓ Firewall rule exists: $($fwRule.DisplayName)" -ForegroundColor Green
    Write-Host "  Enabled: $($fwRule.Enabled)" -ForegroundColor Gray
    Write-Host "  Action: $($fwRule.Action)" -ForegroundColor Gray
} else {
    Write-Host "✗ No firewall rule found for Vision Node" -ForegroundColor Red
    Write-Host ""
    Write-Host "  Creating firewall rule..." -ForegroundColor Yellow
    try {
        New-NetFirewallRule -DisplayName "Vision Node P2P" `
            -Direction Inbound `
            -Protocol TCP `
            -LocalPort $Port `
            -Action Allow `
            -Profile Any `
            -ErrorAction Stop | Out-Null
        Write-Host "  ✓ Firewall rule created successfully" -ForegroundColor Green
    } catch {
        Write-Host "  ✗ Failed to create firewall rule (run as Administrator)" -ForegroundColor Red
    }
}
Write-Host ""

# 6. Test external connectivity
if (![string]::IsNullOrEmpty($PublicIP)) {
    Write-Host "[6] Testing external connectivity to $PublicIP`:$Port..." -ForegroundColor Yellow
    Write-Host "  This requires port forwarding on your router!" -ForegroundColor Gray
    
    # Try to connect from external service
    try {
        $testUrl = "https://api.hackertarget.com/nping/?q=$PublicIP`:$Port"
        $result = Invoke-RestMethod -Uri $testUrl -TimeoutSec 15
        
        if ($result -match "Host is up") {
            Write-Host "✓ Port $Port appears to be open externally" -ForegroundColor Green
        } elseif ($result -match "filtered") {
            Write-Host "⚠ Port $Port is filtered (likely firewalled)" -ForegroundColor Yellow
        } else {
            Write-Host "✗ Port $Port appears to be closed externally" -ForegroundColor Red
            Write-Host "  Result: $result" -ForegroundColor Gray
        }
    } catch {
        Write-Host "⚠ Could not test external connectivity" -ForegroundColor Yellow
        Write-Host "  Manual test: https://www.yougetsignal.com/tools/open-ports/" -ForegroundColor Gray
    }
    Write-Host ""
}

# 7. Check router/NAT configuration
Write-Host "[7] Router/NAT Configuration Check" -ForegroundColor Yellow
Write-Host "  For external connections, you need to:" -ForegroundColor Gray
Write-Host "    1. Log into your router's admin panel" -ForegroundColor Gray
Write-Host "    2. Find 'Port Forwarding' or 'NAT' settings" -ForegroundColor Gray
Write-Host "    3. Forward external port $Port to internal IP + port $Port" -ForegroundColor Gray
Write-Host ""
Write-Host "  Your local IP addresses:" -ForegroundColor Gray
$ips = Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.IPAddress -notlike "127.*" -and $_.IPAddress -notlike "169.*" }
foreach ($ip in $ips) {
    Write-Host "    - $($ip.IPAddress) ($($ip.InterfaceAlias))" -ForegroundColor Gray
}
Write-Host ""

# 8. Generate ngrok command (alternative)
Write-Host "[8] Alternative: Use ngrok for public access" -ForegroundColor Yellow
Write-Host "  If you don't want to configure port forwarding:" -ForegroundColor Gray
Write-Host "    1. Download ngrok: https://ngrok.com/download" -ForegroundColor Gray
Write-Host "    2. Run: ngrok http $Port" -ForegroundColor Gray
Write-Host "    3. Use the provided URL as your node address" -ForegroundColor Gray
Write-Host ""

# 9. Summary and next steps
Write-Host "=== Summary ===" -ForegroundColor Cyan
Write-Host ""
if (![string]::IsNullOrEmpty($PublicIP)) {
    Write-Host "Your P2P address should be:" -ForegroundColor Yellow
    Write-Host "  http://$PublicIP`:$Port" -ForegroundColor White
    Write-Host ""
}
Write-Host "For others to connect to your node:" -ForegroundColor Yellow
Write-Host "  1. Ensure Windows Firewall allows port $Port" -ForegroundColor Gray
Write-Host "  2. Configure router port forwarding: $Port -> (local IP):$Port" -ForegroundColor Gray
Write-Host "  3. Share your P2P address: http://$PublicIP`:$Port" -ForegroundColor Gray
Write-Host "  4. Test from external service: https://www.yougetsignal.com/tools/open-ports/" -ForegroundColor Gray
Write-Host ""

# 10. Test peer add (if they provide a peer URL)
Write-Host "[10] Testing peer connectivity (optional)" -ForegroundColor Yellow
$peerUrl = Read-Host "Enter a peer URL to test connection (or press Enter to skip)"
if (![string]::IsNullOrEmpty($peerUrl)) {
    Write-Host "  Testing connection to $peerUrl..." -ForegroundColor Gray
    try {
        $peerInfo = Invoke-RestMethod -Uri "$peerUrl/info" -Method Get -TimeoutSec 10
        Write-Host "  ✓ Successfully connected to peer" -ForegroundColor Green
        Write-Host "    Peer version: $($peerInfo.version)" -ForegroundColor Gray
        
        # Try to add as peer
        Write-Host "  Adding peer to your node..." -ForegroundColor Gray
        $adminToken = $env:VISION_ADMIN_TOKEN
        if ([string]::IsNullOrEmpty($adminToken)) {
            Write-Host "  ✗ VISION_ADMIN_TOKEN not set (required for /peer/add)" -ForegroundColor Red
        } else {
            try {
                $addResult = Invoke-RestMethod -Uri "$NodeUrl/peer/add?token=$adminToken" `
                    -Method Post `
                    -Body (@{ url = $peerUrl } | ConvertTo-Json) `
                    -ContentType "application/json"
                Write-Host "  ✓ Peer added successfully" -ForegroundColor Green
            } catch {
                Write-Host "  ✗ Failed to add peer: $_" -ForegroundColor Red
            }
        }
    } catch {
        Write-Host "  ✗ Failed to connect to peer: $_" -ForegroundColor Red
    }
}
Write-Host ""

Write-Host "=== Diagnostic Complete ===" -ForegroundColor Cyan
Write-Host ""
