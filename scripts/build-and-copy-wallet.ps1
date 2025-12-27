# Build and Copy Wallet Script
# This builds the React wallet and copies it to where the node serves it from

Write-Host "🔨 Building wallet..." -ForegroundColor Cyan

# Navigate to wallet-marketplace-source to build
Set-Location "c:\vision-node\wallet-marketplace-source"

# Clean old build artifacts first
if (Test-Path "dist") {
    Write-Host "🧹 Cleaning wallet-marketplace-source/dist..." -ForegroundColor Yellow
    Remove-Item -Recurse -Force "dist"
}

# Build the wallet
npm run build

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Build failed!" -ForegroundColor Red
    Set-Location "c:\vision-node"
    exit 1
}

Write-Host "✅ Build successful!" -ForegroundColor Green

# Clean target directory to avoid stale hashed assets
if (Test-Path "c:\vision-node\wallet\dist") {
    Write-Host "🧹 Cleaning c:\vision-node\wallet\dist..." -ForegroundColor Yellow
    Remove-Item -Recurse -Force "c:\vision-node\wallet\dist"
}

# Create target directory
Write-Host "📦 Copying to c:\vision-node\wallet\dist..." -ForegroundColor Cyan
New-Item -ItemType Directory -Path "c:\vision-node\wallet\dist" -Force | Out-Null

# Copy all files including assets folder
Copy-Item -Recurse -Force "dist\*" "c:\vision-node\wallet\dist\"

Set-Location "c:\vision-node"

Write-Host "✅ Wallet updated at: c:\vision-node\wallet\dist" -ForegroundColor Green
Write-Host "✅ Assets: c:\vision-node\wallet\dist\assets\" -ForegroundColor Green
Write-Host ""
Write-Host "Now run your node and visit: http://127.0.0.1:7070/app" -ForegroundColor Yellow
Write-Host "Assets will be served from: /app/assets/*" -ForegroundColor Cyan
