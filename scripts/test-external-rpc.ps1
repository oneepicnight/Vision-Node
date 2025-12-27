# COPILOT TASK: Create PowerShell script to test external RPC wiring
#
# Goal:
# - Provide a simple script scripts/test-external-rpc.ps1 that:
#   * Calls /rpc/status
#   * Prints per-chain status
#   * Optionally exercises a small test RPC (e.g., getblockcount).
#
# Steps:
# 1. Create file scripts/test-external-rpc.ps1.
# 2. Add parameters:
#      param(
#        [string]$NodeUrl = "http://127.0.0.1:7070"
#      )
# 3. Use Invoke-RestMethod to:
#      - GET "$NodeUrl/rpc/status"
#      - Print a table of chain, configured, ok, last_error.
# 4. Optionally, if backend exposes a /rpc/test endpoint later, call it as well.
# 5. Print a final summary with overall PASS/FAIL based on status values.
#
# Result:
# - I can run: `.\scripts\test-external-rpc.ps1 -NodeUrl http://127.0.0.1:7070`
#   and immediately see if BTC/BCH/DOGE RPC wiring is healthy.

param(
    [string]$NodeUrl = "http://127.0.0.1:7070",
    [switch]$Verbose
)

Write-Host "🔍 Testing External RPC Configuration" -ForegroundColor Cyan
Write-Host "Node URL: $NodeUrl" -ForegroundColor Gray
Write-Host ""

# Test /rpc/status endpoint
try {
    Write-Host "Fetching RPC status..." -ForegroundColor Yellow
    $response = Invoke-RestMethod -Uri "$NodeUrl/rpc/status" -Method Get -TimeoutSec 10
    
    if ($null -eq $response) {
        Write-Host "❌ Failed: Empty response from /rpc/status" -ForegroundColor Red
        exit 1
    }
    
    # Parse response
    $chains = @()
    $allOk = $true
    
    foreach ($chain in $response.PSObject.Properties) {
        $chainName = $chain.Name
        $status = $chain.Value
        
        $obj = [PSCustomObject]@{
            Chain = $chainName.ToUpper()
            Configured = if ($status.configured) { "✅ Yes" } else { "❌ No" }
            Status = if ($status.ok) { "✅ OK" } else { "❌ FAIL" }
            Error = if ($status.last_error) { $status.last_error } else { "-" }
        }
        
        $chains += $obj
        
        if (-not $status.ok) {
            $allOk = $false
        }
    }
    
    # Display results as table
    Write-Host ""
    Write-Host "RPC Status Results:" -ForegroundColor Cyan
    $chains | Format-Table -AutoSize
    
    # Summary
    Write-Host ""
    if ($allOk -and $chains.Count -gt 0) {
        Write-Host "✅ PASS: All configured chains are healthy ($($chains.Count) total)" -ForegroundColor Green
        exit 0
    }
    elseif ($chains.Count -eq 0) {
        Write-Host "⚠️  WARNING: No external RPC chains configured" -ForegroundColor Yellow
        Write-Host "   Configure chains in config/external_rpc.toml or via environment variables" -ForegroundColor Gray
        exit 0
    }
    else {
        Write-Host "❌ FAIL: Some chains have errors" -ForegroundColor Red
        Write-Host ""
        Write-Host "Troubleshooting:" -ForegroundColor Yellow
        Write-Host "  1. Check config/external_rpc.toml for correct URLs" -ForegroundColor Gray
        Write-Host "  2. Verify RPC endpoints are reachable" -ForegroundColor Gray
        Write-Host "  3. Check credentials (username/password)" -ForegroundColor Gray
        Write-Host "  4. Review logs for detailed error messages" -ForegroundColor Gray
        exit 1
    }
}
catch {
    Write-Host "❌ FAIL: Could not connect to $NodeUrl/rpc/status" -ForegroundColor Red
    Write-Host "Error: $_" -ForegroundColor Red
    Write-Host ""
    Write-Host "Possible causes:" -ForegroundColor Yellow
    Write-Host "  1. Node not running (start with: cargo run --release)" -ForegroundColor Gray
    Write-Host "  2. Wrong port (check VISION_PORT environment variable)" -ForegroundColor Gray
    Write-Host "  3. Firewall blocking connection" -ForegroundColor Gray
    exit 1
}

# Test oracle endpoint (bonus check)
if ($Verbose) {
    Write-Host ""
    Write-Host "Checking price oracle..." -ForegroundColor Yellow
    
    try {
        $prices = Invoke-RestMethod -Uri "$NodeUrl/oracle/prices" -Method Get -TimeoutSec 10
        
        Write-Host "Oracle Status:" -ForegroundColor Cyan
        Write-Host "  BTC/USD: $($prices.prices.BTCUSD)" -ForegroundColor Gray
        Write-Host "  BCH/USD: $($prices.prices.BCHUSD)" -ForegroundColor Gray
        Write-Host "  DOGE/USD: $($prices.prices.DOGEUSD)" -ForegroundColor Gray
        Write-Host "  Last Update: $($prices.last_update)" -ForegroundColor Gray
        
        if ($prices.stale) {
            Write-Host "  ⚠️  Prices are stale (>5 min old)" -ForegroundColor Yellow
        }
        else {
            Write-Host "  ✅ Prices are fresh" -ForegroundColor Green
        }
    }
    catch {
        Write-Host "  ⚠️  Could not fetch oracle prices (may not be initialized yet)" -ForegroundColor Yellow
    }
}
