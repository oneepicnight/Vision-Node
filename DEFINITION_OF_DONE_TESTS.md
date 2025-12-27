# Definition of Done: Test Suite

After implementing Patches 1-4, validate the implementation using these two focused tests.

## Test A: Clean Local Churn ✓

**What it tests:** Verifies that Patches 1-4 have eliminated dial spam and self-connection attempts while enabling clean churn recovery.

**Run it:**
```powershell
.\test-a-clean-churn.ps1 -SoakSeconds 120 -ChurnIntervalSeconds 30 -ChurnDownSeconds 10
```

**What it does:**
1. Launches 5 nodes with `VISION_LOCAL_TEST=1` and `VISION_PEERBOOK_SCOPE=local-5nodes`
2. Waits for mesh formation (all nodes reach 2+ peers)
3. Performs rolling churn: kill random node for 10s, bring it back, repeat every 30s for 2 minutes
4. During recovery, monitors dial behavior and final peer counts
5. Analyzes logs for problematic patterns

**Expected Results:**
- ✅ All nodes end at `Connected=4` peers (full mesh)
- ✅ NO "already_connected" spam in logs
- ✅ NO self-dial attempts (127.x to 127.x)
- ✅ NO public IP errors (only private IPs in local mode)
- ✅ Dial timeouts ONLY during exact down window
- ✅ Fast recovery: <30s to return to full mesh after restart

**What to check:**
```bash
# Look at logs for clean dial behavior
tail -f run-test-a-7070-*/stderr.log | grep -E "SKIP|dial|timeout"
# Should see mostly:
# - [DIAL] SKIP peer (prefilter): ... (from Patch 3)
# - Connection timeout only during down window
# - NO "already_connected" entries
```

**Failure Analysis:**
- If you see "already_connected spam": Patch 3 not applied correctly
- If you see public IP attempts: Patch 2 not working (VISION_LOCAL_TEST mode)
- If you see self-dial: Patch 3 prefilter not checking loopback
- If nodes don't recover: May need longer timeout or peer book issue

---

## Test B: Persistence Discovery ✓

**What it tests:** Verifies that D and E (seedless nodes) can discover all peers via peer book persistence and gossip, even after a complete restart with NO seed configuration.

**Run it:**
```powershell
.\test-b-persistence.ps1 -DiscoveryTimeoutSeconds 120
```

**What it does:**

**Phase 1: Bootstrap Mesh**
1. Launches A, B, C with seeds to establish full mesh
2. Waits for A-B-C to form mesh (all reach 2+ peers)
3. Peer book gets populated via gossip with all peer information
4. Uses shared scope: `VISION_PEERBOOK_SCOPE=local-5nodes`

**Phase 2: Stop Everything**
1. Kills all 5 nodes
2. Peer book data persists on disk (sled database)

**Phase 3: Restart Without Seeds**
1. Restarts all 5 nodes (reuses same data directories)
2. **D and E have NO seed configuration** (seeds empty)
3. A, B, C restart with their seeds to help bootstrap
4. Monitors D/E peer discovery from peer book only

**Phase 4: Verify Discovery**
1. D and E should reach 4 peers (full mesh) within 120s
2. Tracks exactly when each reached full connectivity
3. Validates peer book persistence is working

**Expected Results:**
- ✅ D reaches 4 peers within 30-90s
- ✅ E reaches 4 peers within 30-90s
- ✅ All 5 nodes end at Connected=4
- ✅ NO seed configuration required for D/E once peer book exists

**What to check:**
```bash
# Watch real-time discovery
tail -f run-test-b-*/stderr.log | grep -E "peer_count|dial|connected"
# Should see D/E discovering peers progressively without seeds
```

**Success Indicators:**
- D/E show increasing peer counts: 0 → 1 → 2 → 3 → 4
- No timeout errors for D/E during discovery
- Discovery happens quickly (within 60s is ideal)
- All nodes stable at 4 peers

**Failure Analysis:**
- If D/E don't discover: Peer book not persisting or gossip broken
- If discovery takes >90s: Peer book reads slow or gossip rate too low
- If nodes show 0 peers: Data directory not reused or scope mismatch

---

## Quick Test: Verify Patch Integration

Run both tests back-to-back to validate all patches work together:

```powershell
# Build fresh
cargo build --release

# Test A: Clean churn behavior
.\test-a-clean-churn.ps1 -SoakSeconds 120

# Wait for logs
Start-Sleep -Seconds 10

# Test B: Persistence + discovery
.\test-b-persistence.ps1 -DiscoveryTimeoutSeconds 120
```

## Expected Timeline

- **Test A**: 3-4 minutes (30s startup + 2min churn + 30s recovery + analysis)
- **Test B**: 4-5 minutes (30s bootstrap + 5s stop + 30s restart + 2min discovery + analysis)
- **Total**: ~10 minutes for full validation

## Success Criteria Summary

| Criterion | Test A | Test B |
|-----------|--------|--------|
| All nodes reach 4 peers | ✅ Required | ✅ Required |
| No "already_connected" spam | ✅ Required | N/A |
| No self-dial attempts | ✅ Required | N/A |
| Only private IPs in peer store | ✅ Required | ✅ Required |
| Mesh recovers <30s after churn | ✅ Required | N/A |
| D/E discover via persistence | N/A | ✅ Required (<90s) |
| Peer book scoped correctly | ✅ Verified | ✅ Verified |

## Debugging

If either test fails, check:

1. **Build verification:**
   ```powershell
   cargo build --release  # Should succeed
   Get-ChildItem .\target\release\vision-node.exe
   ```

2. **Environment vars applied:**
   ```powershell
   # Check generated launch script
   Get-Content run-test-a-7070-*/launch.ps1 | Select-Object -First 20
   # Should include VISION_LOCAL_TEST=1 and VISION_PEERBOOK_SCOPE=local-5nodes
   ```

3. **Logs analysis:**
   ```powershell
   # Full stderr from a node
   Get-Content run-test-a-7070-*/stderr.log | tail -200
   
   # Search for specific issues
   Get-Content run-test-a-7070-*/stderr.log | Select-String "ERROR|FAIL|panic"
   ```

4. **Peer book persistence:**
   ```powershell
   # Check sled database was created
   Get-ChildItem run-test-b-7070-*/data/
   # Should show vision_data.db and other sled files
   ```

---

## Next Steps After Passing

Once both tests pass ✅:
- Patches 1-4 are complete and working
- Ready for Test 0 (baseline mesh soak)
- Ready for Test 1 (hub node kill/restart)
- Ready for production churn testing

**Summary:** These tests verify that the four patches work together correctly to provide:
1. **Scoped peer books** preventing cross-network pollution
2. **Local test mode** rejecting public IPs
3. **Clean dial candidate prefiltering** reducing spam
4. **Persistent peer discovery** enabling seedless nodes to find peers
