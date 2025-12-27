# 🚦 GO/NO-GO CHECKLIST - Testnet 1 Launch

**Target Date**: November 1, 2025  
**Version**: v0.1.0-testnet1  
**Build**: FULL-only (single-world)

---

## ✅ Build & Package (8/8 Complete)

- [x] `cargo build --release` - 0 errors
- [x] Warnings acceptable (6 remaining, all non-critical)
- [x] Windows ZIP created (10.45 MB)
- [x] Linux tarball script ready
- [x] SHA256 checksums generated
- [x] VERSION file created (`v0.1.0-testnet1`)
- [x] CI/CD workflow configured
- [x] Release templates prepared

**Status**: ✅ **GO**

---

## 🧪 Core Functionality (Pending Tests)

### Basic Operations
- [ ] Node starts without errors
- [ ] `/status` returns valid JSON
- [ ] `/health` returns 200 OK
- [ ] `/metrics` exposes Prometheus data

**Test Command:**
```powershell
# Extract package
Expand-Archive dist\VisionNode-v0.1.0-testnet1-WIN64.zip -DestinationPath test-run
cd test-run\VisionNode-v0.1.0-testnet1-WIN64

# Set environment
$env:VISION_ADMIN_TOKEN = "test-token-123"
$env:VISION_ALLOW_SEED = "1"

# Run node
.\vision-node.exe --port 7070 --data .\data

# In another terminal
curl http://localhost:7070/status
curl http://localhost:7070/health
curl http://localhost:7070/metrics
```

---

## 💰 Wallet & Transfers

- [ ] Seed balance succeeds
- [ ] Transfer creates transaction
- [ ] Balance updates correctly
- [ ] Receipt recorded

**Test Command:**
```powershell
$headers = @{"X-Vision-Admin-Token"="test-token-123"; "Content-Type"="application/json"}

# Seed
Invoke-RestMethod -Uri http://localhost:7070/admin/seed-balance `
  -Method Post -Headers $headers `
  -Body '{"addr":"ALICE","amount":"100000"}' | ConvertTo-Json

# Check balance
$bal = Invoke-RestMethod -Uri http://localhost:7070/balance/ALICE
Write-Host "ALICE balance: $bal"

# Transfer
$tx = Invoke-RestMethod -Uri http://localhost:7070/wallet/transfer `
  -Method Post -ContentType "application/json" `
  -Body '{"from":"ALICE","to":"BOB","amount":"2500"}' | ConvertTo-Json
Write-Host "Transfer TX: $($tx.tx_hash)"

# Check receipt
$receipts = Invoke-RestMethod -Uri http://localhost:7070/receipts/latest?limit=5
Write-Host "Receipts found: $($receipts.receipts.Count)"
```

**Status**: ⏳ **PENDING**

---

## 📊 Prometheus Metrics

- [ ] `/metrics` shows `vision_transfers_total`
- [ ] Counters increment correctly
- [ ] No duplicate metric names

**Test Command:**
```powershell
$metrics = Invoke-WebRequest -Uri http://localhost:7070/metrics
if ($metrics.Content -match 'vision_transfers_total\s+(\d+)') {
    Write-Host "✅ Transfers metric: $($Matches[1])"
} else {
    Write-Host "❌ Transfers metric not found"
}
```

**Status**: ⏳ **PENDING**

---

## 💥 Burst Test (1000 Transfers)

- [ ] 1000 transfers complete without panics
- [ ] No deadlocks or hangs
- [ ] Metrics counters accurate
- [ ] Memory usage stable

**Test Command:**
```powershell
.\scripts\burst-test.ps1
```

**Expected**:
- Success rate > 95%
- Duration < 60 seconds
- No panics or crashes
- Metrics show correct count

**Status**: ⏳ **PENDING**

---

## 🏦 Vault & Market (Critical)

- [ ] Market sale creates 4 receipts (buyer, vault, fund, founder x2)
- [ ] Vault balance increments by 50%
- [ ] Fund balance increments by 30%  
- [ ] Founder balances increment by 10% each
- [ ] Epoch payout writes `vault_payout` receipts

**Test Command:**
```powershell
# Enable market in config
# Create test land listing
# Buy land
# Check receipts

$receipts = Invoke-RestMethod -Uri http://localhost:7070/receipts/latest?limit=10
$marketReceipts = $receipts.receipts | Where-Object {$_.memo -like "*market*"}
Write-Host "Market receipts: $($marketReceipts.Count)"

# Should see 4: buyer (transfer), vault (receive), fund (receive), founders (receive x2)
```

**Status**: ⏳ **PENDING**

---

## 📦 Package Integrity

- [x] Windows ZIP extracts cleanly
- [x] SHA256 checksum matches
- [x] Binary is executable
- [ ] No missing dependencies
- [ ] Run script works

**Test Command:**
```powershell
# Verify checksum
$expectedHash = Get-Content dist\VisionNode-v0.1.0-testnet1-WIN64.zip.sha256
$actualHash = (Get-FileHash dist\VisionNode-v0.1.0-testnet1-WIN64.zip).Hash
if ($expectedHash.Trim() -eq $actualHash) {
    Write-Host "✅ Checksum verified"
} else {
    Write-Host "❌ Checksum mismatch!"
}

# Extract
Expand-Archive dist\VisionNode-v0.1.0-testnet1-WIN64.zip -DestinationPath test-extract -Force

# Run
cd test-extract\VisionNode-v0.1.0-testnet1-WIN64
.\run.bat
```

**Status**: ⏳ **PENDING**

---

## 📝 Documentation

- [x] `VERSION` file present
- [x] Release notes written
- [x] API documentation exists (`docs/MVP_ENDPOINTS.md`)
- [x] Build instructions clear (`BUILD_VARIANTS.md`)
- [x] Security warnings included

**Status**: ✅ **GO**

---

## 🔐 Security Review

- [x] Admin token required for sensitive endpoints
- [x] Seed endpoint disabled by default
- [x] CORS properly configured
- [ ] No hardcoded credentials
- [ ] Rate limiting functional

**Checklist**:
- Admin endpoints require `X-Vision-Admin-Token`
- Seed requires `VISION_ALLOW_SEED=1`
- Strong token recommended in docs
- CORS respects `VISION_CORS_ORIGINS`

**Status**: ⏳ **PENDING VERIFICATION**

---

## 🌐 P2P & Networking

- [ ] Peer discovery works
- [ ] Block gossip succeeds
- [ ] Transaction propagation works
- [ ] Sync protocol functional

**Test Command**:
```powershell
# Start first node
.\vision-node.exe --port 7070 --data .\data-node1

# Start second node
.\vision-node.exe --port 7071 --data .\data-node2

# Add peer
curl http://localhost:7071/peer/add -X POST -d '{"url":"http://localhost:7070"}'

# Check peers
curl http://localhost:7071/peers
```

**Status**: ⏳ **PENDING**

---

## 📈 Performance

- [ ] Block production < 1 second
- [ ] Transfer processing < 100ms
- [ ] API response times < 200ms
- [ ] Memory usage < 500MB
- [ ] No memory leaks

**Status**: ⏳ **PENDING**

---

## 🚨 Known Issues

### Critical (Blockers)
- None identified

### High (Should Fix)
- None identified

### Medium (Can Ship With)
- 6 compiler warnings (unused parentheses, unused assignments)
- Some endpoints may be experimental/undocumented
- WebSocket reconnection may need client-side logic

### Low (Nice to Have)
- Module extraction deferred to post-testnet
- Some experimental features incomplete

---

## 📊 GO/NO-GO Decision Matrix

| Category | Status | Weight | Pass? |
|----------|--------|--------|-------|
| **Build & Package** | ✅ Complete | Critical | ✅ YES |
| **Core Functionality** | ⏳ Pending | Critical | ⏳ TESTING |
| **Wallet & Transfers** | ⏳ Pending | Critical | ⏳ TESTING |
| **Metrics** | ⏳ Pending | High | ⏳ TESTING |
| **Burst Test** | ⏳ Pending | High | ⏳ TESTING |
| **Vault & Market** | ⏳ Pending | Critical | ⏳ TESTING |
| **Package Integrity** | ⏳ Pending | Critical | ⏳ TESTING |
| **Documentation** | ✅ Complete | High | ✅ YES |
| **Security** | ⏳ Pending | Critical | ⏳ TESTING |
| **P2P** | ⏳ Pending | Medium | ⏳ TESTING |
| **Performance** | ⏳ Pending | Medium | ⏳ TESTING |

---

## 🎯 GO Decision Criteria

### MUST PASS (Critical)
1. ✅ Build & Package complete
2. ⏳ Core functionality working (status, health, metrics)
3. ⏳ Wallet transfers succeed
4. ⏳ Vault 50/30/20 split verified
5. ⏳ Package integrity confirmed
6. ⏳ Security review passed

### SHOULD PASS (High Priority)
1. ⏳ Burst test (1000 transfers) succeeds
2. ⏳ Prometheus metrics accurate
3. ✅ Documentation complete

### NICE TO HAVE (Medium Priority)
1. ⏳ P2P networking tested
2. ⏳ Performance benchmarks run
3. ⏳ Epoch payouts verified

---

## 🚀 Launch Sequence

### Pre-Launch (1-2 hours)
1. [ ] Run smoke tests
2. [ ] Run burst test
3. [ ] Verify vault mechanics
4. [ ] Test package extraction
5. [ ] Review security settings

### Launch (30 minutes)
1. [ ] Create GitHub tag `v0.1.0-testnet1`
2. [ ] Push tag to trigger CI/CD
3. [ ] Verify CI builds complete
4. [ ] Download artifacts
5. [ ] Upload to GitHub Release
6. [ ] Publish release

### Post-Launch (2-4 hours)
1. [ ] Post to BitcoinTalk
2. [ ] Post to Reddit (r/CryptoCurrency, r/rust, r/gamedev)
3. [ ] Tweet announcement thread
4. [ ] Monitor for issues
5. [ ] Respond to community feedback

---

## ✅ Final GO/NO-GO

**Current Status**: ⏳ **TESTING IN PROGRESS**

**Recommendation**: 
- Complete the PENDING tests above
- If all critical tests PASS → ✅ **GO FOR LAUNCH**
- If any critical tests FAIL → ❌ **NO-GO** (fix and retest)

**Estimated Time to Launch**:
- Testing: 1-2 hours
- If GO: Launch same day
- If NO-GO: Fix issues, retest (1-3 days)

---

**Last Updated**: November 1, 2025  
**Next Review**: After smoke tests complete  
**Decision Owner**: Project Lead
