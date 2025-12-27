$ErrorActionPreference = "Stop"
$ver = (Get-Content VERSION).Trim()
Write-Host "Building v$ver..." -ForegroundColor Green
cargo build --release
mkdir -Force dist/VisionNode-$ver-WIN64 | Out-Null
Copy-Item target/release/vision-node.exe dist/VisionNode-$ver-WIN64/
Copy-Item VERSION dist/VisionNode-$ver-WIN64/
"vision-node.exe --port 7070" | Out-File dist/VisionNode-$ver-WIN64/run.bat -Encoding ascii
Compress-Archive -Force -Path dist/VisionNode-$ver-WIN64/* -DestinationPath dist/VisionNode-$ver-WIN64.zip
Write-Host "Created dist/VisionNode-$ver-WIN64.zip" -ForegroundColor Green
