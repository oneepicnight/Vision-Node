param(
  [int]$HttpPort = 7070,
  [int]$P2pPort = 7072,
  [string]$PublicIp,
  [int]$PublicPort,
  [string]$AnchorSeeds,
  [string]$SeedPeersPath
)

Write-Host "[RUN] VisionNode WAN runner starting..." -ForegroundColor Cyan

# Apply env overrides if provided
if ($HttpPort) { $env:VISION_PORT = "$HttpPort" }
if ($P2pPort) { $env:VISION_P2P_PORT = "$P2pPort" }
if ($PublicIp) { $env:VISION_PUBLIC_IP = $PublicIp }
if ($PublicPort) { $env:VISION_PUBLIC_PORT = "$PublicPort" }
if ($AnchorSeeds) { $env:VISION_ANCHOR_SEEDS = $AnchorSeeds }

# Optional: seed peers JSON override
if ($SeedPeersPath) {
  if (Test-Path $SeedPeersPath) {
    $dest = Join-Path $PSScriptRoot "seed_peers.json"
    Copy-Item -Path $SeedPeersPath -Destination $dest -Force
    Write-Host "[RUN] Using seed peers from: $SeedPeersPath" -ForegroundColor Green
  } else {
    Write-Host "[RUN] SeedPeersPath not found: $SeedPeersPath" -ForegroundColor Yellow
  }
}

# Show effective config
Write-Host "[RUN] HTTP Port: $($env:VISION_PORT)" -ForegroundColor Gray
Write-Host "[RUN] P2P Port: $($env:VISION_P2P_PORT)" -ForegroundColor Gray
Write-Host "[RUN] Public IP: $($env:VISION_PUBLIC_IP)" -ForegroundColor Gray
Write-Host "[RUN] Public Port: $($env:VISION_PUBLIC_PORT)" -ForegroundColor Gray
Write-Host "[RUN] Anchor Seeds: $($env:VISION_ANCHOR_SEEDS)" -ForegroundColor Gray

# Launch the node
$exe = Join-Path $PSScriptRoot "vision-node.exe"
if (!(Test-Path $exe)) {
  Write-Host "[RUN] vision-node.exe not found next to script." -ForegroundColor Red
  exit 2
}

Write-Host "[RUN] Starting vision-node.exe..." -ForegroundColor Cyan
& $exe
