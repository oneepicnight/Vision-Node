# Configure P2P Peer for Local Network Testing
# Run this on BOTH the public node AND the miner node

param(
    [Parameter(Mandatory=$true)]
    [string]$PeerIP,  # The IP of the OTHER node
    
    [int]$PeerPort = 7070,
    
    [int]$LocalPort = 7070
)

$dataDir = ".\vision_data_$LocalPort"
$configFile = Join-Path $dataDir "node_peer.json"

# Create data directory if it doesn't exist
if (!(Test-Path $dataDir)) {
    New-Item -ItemType Directory -Path $dataDir | Out-Null
    Write-Host "✅ Created data directory: $dataDir" -ForegroundColor Green
}

# Create peer configuration (P2P uses HTTP port + 1)
$p2pPort = $PeerPort + 1
$config = @{
    p2p_peer = "${PeerIP}:${p2pPort}"
} | ConvertTo-Json

# Write configuration file
$config | Set-Content -Path $configFile -Encoding UTF8

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "✅ P2P Peer Configuration Created" -ForegroundColor Green
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "File: $configFile" -ForegroundColor Yellow
Write-Host "Peer: ${PeerIP}:${p2pPort} (P2P port = HTTP port + 1)" -ForegroundColor Yellow
Write-Host ""
Write-Host "Start this node with:" -ForegroundColor White
Write-Host "  .\vision-node.exe" -ForegroundColor Cyan
Write-Host ""
Write-Host "The node will automatically connect to ${PeerIP}:${p2pPort} on startup" -ForegroundColor Gray
Write-Host "Note: P2P uses port $p2pPort (HTTP: $PeerPort, P2P: $p2pPort)" -ForegroundColor Gray
Write-Host ""
