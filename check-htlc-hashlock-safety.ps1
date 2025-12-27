# CI Safety Check: HTLC Hash Lock Must Use SHA256 (Never BLAKE3)
# ================================================================
# This script scans the codebase to ensure HTLC hash locks use SHA256
# for cross-chain compatibility. BLAKE3 is allowed ONLY for internal
# identifiers (htlc_id), never for the cryptographic lock itself.

$ErrorActionPreference = "Stop"
$failed = $false

Write-Host ""
Write-Host "HTLC Hash Lock Safety Check (SHA256 Required)" -ForegroundColor Cyan
Write-Host "======================================================================" -ForegroundColor Cyan

# ============================================================================
# CHECK 1: Verify hashlock.rs uses SHA256
# ============================================================================
Write-Host ""
Write-Host "Checking src\swap\hashlock.rs uses SHA256..." -ForegroundColor Yellow

$hashlockFile = "src\swap\hashlock.rs"
if (!(Test-Path $hashlockFile)) {
    Write-Host "  FAIL: File not found: $hashlockFile" -ForegroundColor Red
    Write-Host "     Hash lock functions must exist in dedicated module." -ForegroundColor Red
    $failed = $true
} else {
    $content = Get-Content $hashlockFile -Raw
    
    # Must use sha2 crate with Sha256
    if ($content -like "*use sha2*Sha256*") {
        Write-Host "  PASS: Uses sha2 crate with Sha256" -ForegroundColor Green
    } else {
        Write-Host "  FAIL: Missing sha2 Sha256 import" -ForegroundColor Red
        $failed = $true
    }
    
    # Must have htlc_hash_lock function
    if ($content -like "*pub fn htlc_hash_lock*") {
        Write-Host "  PASS: Function htlc_hash_lock() exists" -ForegroundColor Green
    } else {
        Write-Host "  FAIL: Missing pub fn htlc_hash_lock()" -ForegroundColor Red
        $failed = $true
    }
    
    # Must have htlc_hash_lock_hex function
    if ($content -like "*pub fn htlc_hash_lock_hex*") {
        Write-Host "  PASS: Function htlc_hash_lock_hex() exists" -ForegroundColor Green
    } else {
        Write-Host "  FAIL: Missing pub fn htlc_hash_lock_hex()" -ForegroundColor Red
        $failed = $true
    }
    
    # Check for BLAKE3 usage in actual code (not comments or test strings)
    $codeLines = Get-Content $hashlockFile | Where-Object { 
        $_ -notmatch '^\s*//' -and 
        $_ -notmatch '^\s*\*' -and
        $_ -notmatch 'should_panic' -and
        $_ -notmatch 'test_never_use_blake3' -and
        $_ -notmatch 'contains.*blake3' -and
        $_ -notmatch 'panic!.*BLAKE3'
    }
    $blake3InCode = $codeLines | Where-Object { $_ -match 'blake3::|use blake3' }
    
    if ($blake3InCode) {
        Write-Host "  FAIL: BLAKE3 import or usage found in actual code" -ForegroundColor Red
        $failed = $true
    } else {
        Write-Host "  PASS: No BLAKE3 imports in hash lock implementation" -ForegroundColor Green
    }
}

# ============================================================================
# CHECK 2: Verify atomic_swaps.rs uses swap module
# ============================================================================
Write-Host ""
Write-Host "Checking src\atomic_swaps.rs uses swap module..." -ForegroundColor Yellow

$atomicFile = "src\atomic_swaps.rs"
if (!(Test-Path $atomicFile)) {
    Write-Host "  SKIP: File not found: $atomicFile" -ForegroundColor Yellow
} else {
    $content = Get-Content $atomicFile -Raw
    
    # Must use swap module hash lock
    if ($content -like "*crate::swap::htlc_hash_lock_hex*") {
        Write-Host "  PASS: Uses crate::swap::htlc_hash_lock_hex()" -ForegroundColor Green
    } else {
        Write-Host "  FAIL: Not using swap::htlc_hash_lock_hex()" -ForegroundColor Red
        Write-Host "     Must centralize hash lock via swap module." -ForegroundColor Red
        $failed = $true
    }
}

# ============================================================================
# CHECK 3: Verify main.rs claim_htlc uses swap module
# ============================================================================
Write-Host ""
Write-Host "Checking src\main.rs claim_htlc uses swap module..." -ForegroundColor Yellow

$mainFile = "src\main.rs"
if (!(Test-Path $mainFile)) {
    Write-Host "  SKIP: File not found: $mainFile" -ForegroundColor Yellow
} else {
    $content = Get-Content $mainFile -Raw
    
    # Must document htlc_id is internal only
    if ($content -like "*internal identifier*") {
        Write-Host "  PASS: htlc_id documented as internal identifier" -ForegroundColor Green
    } else {
        Write-Host "  WARN: Missing documentation: htlc_id = internal only" -ForegroundColor Yellow
    }
    
    # Must use swap::verify_hash_lock_hex in claim_htlc
    if ($content -like "*swap::verify_hash_lock_hex*") {
        Write-Host "  PASS: Uses swap::verify_hash_lock_hex()" -ForegroundColor Green
    } else {
        Write-Host "  FAIL: claim_htlc not using swap::verify_hash_lock_hex()" -ForegroundColor Red
        $failed = $true
    }
}

# ============================================================================
# Final Result
# ============================================================================
Write-Host ""
if ($failed) {
    Write-Host "FAIL: HTLC HASH LOCK SAFETY CHECK FAILED" -ForegroundColor Red
    Write-Host "======================================================================" -ForegroundColor Red
    Write-Host "BLAKE3 breaks atomic swap interoperability." -ForegroundColor Red
    exit 1
} else {
    Write-Host "PASS: HTLC HASH LOCK SAFETY CHECK PASSED" -ForegroundColor Green
    Write-Host "======================================================================" -ForegroundColor Green
    Write-Host "All hash locks use SHA256 for cross-chain compatibility." -ForegroundColor Green
    exit 0
}
