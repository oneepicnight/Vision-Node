# Guardian Control Room - Initial Setup Script
# 
# This script helps you configure your creator wallet address for Guardian access.

param(
    [Parameter(Mandatory=$false)]
    [string]$CreatorAddress
)

Write-Host "🛡️ Guardian Control Room - Initial Setup" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check if creator address provided
if (-not $CreatorAddress) {
    Write-Host "Usage: .\setup-guardian.ps1 -CreatorAddress '0xYourWalletAddressHere'" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "This script configures which wallet address can access the Guardian Control Room." -ForegroundColor Gray
    Write-Host ""
    
    $CreatorAddress = Read-Host "Enter your creator wallet address"
    
    if (-not $CreatorAddress) {
        Write-Host "❌ No address provided. Exiting." -ForegroundColor Red
        exit 1
    }
}

Write-Host ""
Write-Host "Creator Address: $CreatorAddress" -ForegroundColor White
Write-Host ""
Write-Host "⚠️  IMPORTANT:" -ForegroundColor Yellow
Write-Host "  - Only this wallet can access /guardian" -ForegroundColor Yellow
Write-Host "  - This grants god-tier controls (CASH airdrop, etc.)" -ForegroundColor Yellow
Write-Host "  - Store this address securely" -ForegroundColor Yellow
Write-Host ""

$confirm = Read-Host "Confirm? (yes/no)"

if ($confirm -ne "yes") {
    Write-Host "❌ Setup cancelled." -ForegroundColor Red
    exit 0
}

Write-Host ""
Write-Host "📝 Creating configuration..." -ForegroundColor Cyan

# Create a temporary Rust file to initialize the creator config
$initScript = @"
use sled::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open database
    let db = Config::new()
        .path("./data")
        .open()?;
    
    // Initialize creator config
    vision_node::guardian::initialize_creator_config(&db, "$CreatorAddress")?;
    
    println!("✅ Creator address configured: $CreatorAddress");
    println!("🛡️ Guardian Control Room is now locked to this wallet.");
    
    Ok(())
}
"@

# Save to temp file
$tempFile = "temp_init_creator.rs"
$initScript | Out-File -FilePath $tempFile -Encoding UTF8

Write-Host "✅ Configuration script created" -ForegroundColor Green
Write-Host ""
Write-Host "🚀 Next Steps:" -ForegroundColor Cyan
Write-Host ""
Write-Host "1. Start your Vision Node:" -ForegroundColor White
Write-Host "   .\vision-node.exe --mode guardian" -ForegroundColor Gray
Write-Host ""
Write-Host "2. Login to Vision Guard with wallet:" -ForegroundColor White
Write-Host "   $CreatorAddress" -ForegroundColor Gray
Write-Host ""
Write-Host "3. Navigate to Guardian Control Room:" -ForegroundColor White
Write-Host "   http://localhost:3001/guardian" -ForegroundColor Gray
Write-Host ""
Write-Host "4. If you see the control room, setup successful! 🎉" -ForegroundColor White
Write-Host ""
Write-Host "Note: The creator address is stored in the sled database" -ForegroundColor DarkGray
Write-Host "      at: ./data/creator_config/creator" -ForegroundColor DarkGray
Write-Host ""
Write-Host "To change the creator address later, run this script again." -ForegroundColor DarkGray
Write-Host ""
Write-Host "════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""
Write-Host "🛡️ Your throne awaits, creator." -ForegroundColor Cyan
Write-Host ""

# Clean up temp file
if (Test-Path $tempFile) {
    Remove-Item $tempFile
}
