# Export Bootstrap Block Hashes
# 
# This script queries a running Vision Node for the first 10 blocks
# and formats them for insertion into vision_constants.rs
#
# Usage:
#   .\export-bootstrap-hashes.ps1
#   .\export-bootstrap-hashes.ps1 -Port 7070
#   .\export-bootstrap-hashes.ps1 -Port 7071 -OutputFile bootstrap-hashes.txt

param(
    [int]$Port = 7070,
    [string]$OutputFile = ""
)

$url = "http://localhost:$Port/chain/blocks?from=0&to=9"

Write-Host "🔍 Fetching blocks 0-9 from http://localhost:$Port..." -ForegroundColor Cyan

try {
    $response = Invoke-RestMethod -Uri $url -Method Get -ErrorAction Stop
    $blocks = $response.blocks
    
    if ($blocks.Count -ne 10) {
        Write-Host "❌ Error: Expected 10 blocks, got $($blocks.Count)" -ForegroundColor Red
        Write-Host "   Make sure the node has mined at least 10 blocks." -ForegroundColor Yellow
        exit 1
    }
    
    Write-Host "✅ Retrieved 10 blocks successfully`n" -ForegroundColor Green
    
    # Build output
    $output = @"
// ================================================================================
// BOOTSTRAP CHECKPOINT - Baked-in prefix for network quarantine
// ================================================================================
// Generated: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
// Source: http://localhost:$Port
// ================================================================================

/// Height of the last baked-in bootstrap block (0-based, so 9 = 10 blocks)
pub const BOOTSTRAP_CHECKPOINT_HEIGHT: u64 = 9;

/// Hash of the last baked-in block at height 9.
pub const BOOTSTRAP_CHECKPOINT_HASH: &str =
    "$($blocks[9].header.pow_hash)";

/// All 10 bootstrap block hashes (heights 0-9)
/// These define the canonical start of the chain for this testnet.
pub const BOOTSTRAP_BLOCK_HASHES: [&str; 10] = [
"@

    for ($i = 0; $i -lt 10; $i++) {
        $hash = $blocks[$i].header.pow_hash
        $height = $blocks[$i].header.number
        $timestamp = $blocks[$i].header.timestamp
        $date = [DateTimeOffset]::FromUnixTimeSeconds($timestamp).ToString("yyyy-MM-dd HH:mm:ss")
        
        Write-Host "  Block $height`: $hash" -ForegroundColor Gray
        Write-Host "           Timestamp: $timestamp ($date)" -ForegroundColor DarkGray
        
        if ($i -lt 9) {
            $output += "    `"$hash`", // h=$i`n"
        } else {
            $output += "    `"$hash`", // h=$i (checkpoint)`n"
        }
    }
    
    $output += "];"
    
    Write-Host ""
    Write-Host "📋 Generated constants:" -ForegroundColor Cyan
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
    Write-Host $output
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
    Write-Host ""
    
    if ($OutputFile) {
        $output | Out-File -FilePath $OutputFile -Encoding UTF8
        Write-Host "✅ Written to: $OutputFile" -ForegroundColor Green
    }
    
    Write-Host "📝 Next steps:" -ForegroundColor Yellow
    Write-Host "   1. Copy the generated constants above" -ForegroundColor White
    Write-Host "   2. Open src/vision_constants.rs" -ForegroundColor White
    Write-Host "   3. Replace the BOOTSTRAP_* constants (around line 245)" -ForegroundColor White
    Write-Host "   4. Run: cargo build --release" -ForegroundColor White
    Write-Host "   5. Distribute the new binary" -ForegroundColor White
    Write-Host ""
    
    # Verification
    Write-Host "🔐 Verification:" -ForegroundColor Cyan
    Write-Host "   Checkpoint Height: 9" -ForegroundColor White
    Write-Host "   Checkpoint Hash:   $($blocks[9].header.pow_hash)" -ForegroundColor White
    Write-Host "   Genesis Hash:      $($blocks[0].header.pow_hash)" -ForegroundColor White
    
} catch {
    Write-Host "❌ Error: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host ""
    Write-Host "Troubleshooting:" -ForegroundColor Yellow
    Write-Host "  1. Make sure Vision Node is running on port $Port" -ForegroundColor White
    Write-Host "  2. Check if the node has mined at least 10 blocks" -ForegroundColor White
    Write-Host "  3. Verify the API endpoint is accessible:" -ForegroundColor White
    Write-Host "     curl http://localhost:$Port/chain/status" -ForegroundColor Gray
    exit 1
}
