param(
    [int]$HttpPort = 7070,
    [int]$P2PPort  = 7072,
    [string]$Seeds = "127.0.0.1:8082,127.0.0.1:9092",
    [string]$DataDir = "data"
)

. "$PSScriptRoot\env.ps1"

$env:VISION_HTTP_PORT = $HttpPort.ToString()
$env:VISION_P2P_PORT  = $P2PPort.ToString()
$env:VISION_P2P_SEEDS = $Seeds
$env:VISION_DATA_DIR  = (Join-Path $PSScriptRoot $DataDir)

New-Item -ItemType Directory -Path $env:VISION_DATA_DIR -Force | Out-Null

& "$PSScriptRoot\vision-node.exe" run
