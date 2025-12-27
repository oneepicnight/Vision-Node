# Pending Rewards - Quick Testing Guide

## Test Setup: 3-Node Mining Testnet

### Prerequisites
- 3 nodes compiled with latest code
- All nodes synced to same height
- Mining system active (250ms ticks)

## Test 1: Bank Rewards Without Wallet
**Goal**: Verify rewards are banked when node wins without payout address

### Steps:
1. Start Node 1 **WITHOUT** configuring wallet address
   ```powershell
   .\vision-node.exe
   ```

2. Open panel: http://localhost:7070/panel
   - Verify "Linked Wallet: None"
   - Verify "Pending Rewards: 0 LAND"

3. Wait for Node 1 to win a mining slot
   - Look for logs: `[MINING] âœ… WON SLOT`

4. Check logs for banking confirmation:
   ```
   [PENDING_REWARDS] ðŸ’° Banked 1000000000000000000 for node_id=xxx (no payout address)
   ```

5. Refresh panel â†’ Verify "Pending Rewards: 1.0000 LAND" (yellow color)

6. Check Vault balance increased:
   ```powershell
   curl http://localhost:7070/api/balance/Vault_address
   ```

**Expected Result**:
- âœ… Reward minted to Vault
- âœ… Pending rewards tracked in sled DB
- âœ… UI shows pending amount in yellow

---

## Test 2: Immediate Payout on Configuration
**Goal**: Verify payout happens immediately when wallet is set

### Steps:
1. Node 1 has pending rewards (from Test 1)

2. Configure wallet address via panel:
   - Enter wallet address: `0x1234...`
   - Click "Link Node to Wallet"

3. Check logs for payout:
   ```
   [PENDING_REWARDS] ðŸ’° Attempting payout of 1000000000000000000 to 0x1234... for node_id=xxx
   [PENDING_REWARDS] âœ… Successfully paid out 1000000000000000000 to 0x1234...
   ```

4. Verify panel updates:
   - "Pending Rewards: 0 LAND" (grey)
   - "Linked Wallet: 0x1234..."

5. Check user balance:
   ```powershell
   curl http://localhost:7070/api/balance/0x1234...
   ```

**Expected Result**:
- âœ… Direct transfer from Vaultâ†’User
- âœ… Pending rewards cleared
- âœ… User balance = reward amount
- âœ… Vault balance decreased

---

## Test 3: Retry Loop Recovery
**Goal**: Verify retry loop pays out pending rewards

### Steps:
1. Start Node 2 with wallet already configured:
   ```powershell
   .\vision-node.exe --miner-address 0xABCD...
   ```

2. Manually add pending rewards to test retry (requires code modification):
   - OR: Win mining, immediately remove wallet, win again, re-add wallet

3. Alternative: Start without wallet, win slot, configure wallet, then kill node before payout

4. Restart node with wallet configured

5. Wait 30 seconds for retry loop

6. Check logs:
   ```
   [PENDING_REWARDS] ðŸŽ‰ Retry loop paid out 1000000000000000000 to 0xABCD...
   ```

**Expected Result**:
- âœ… Retry loop detects pending + configured address
- âœ… Automatic payout after 30 seconds
- âœ… No user interaction needed

---

## Test 4: Multiple Wins Accumulation
**Goal**: Verify multiple wins accumulate correctly

### Steps:
1. Start Node 3 without wallet

2. Let it win **3 mining slots**

3. After each win, verify panel shows accumulated amount:
   - Win 1: "Pending Rewards: 1.0000 LAND"
   - Win 2: "Pending Rewards: 2.0000 LAND"
   - Win 3: "Pending Rewards: 3.0000 LAND"

4. Configure wallet address

5. Check single payout for total:
   ```
   [PENDING_REWARDS] âœ… Successfully paid out 3000000000000000000 to 0x...
   ```

**Expected Result**:
- âœ… Each win calls `pending_add()`
- âœ… Total accumulates in sled DB
- âœ… Single payout transfers all at once
- âœ… UI updates show running total

---

## Test 5: Restart Persistence
**Goal**: Verify pending rewards survive node restart

### Steps:
1. Node has pending rewards (e.g., 2.5 LAND)

2. Note the amount shown in panel

3. **Kill the node** (Ctrl+C)

4. Restart node:
   ```powershell
   .\vision-node.exe
   ```

5. Open panel â†’ Verify "Pending Rewards: 2.5000 LAND" still shows

6. Configure wallet â†’ Verify payout succeeds

**Expected Result**:
- âœ… sled DB persists across restarts
- âœ… Pending rewards not lost
- âœ… Payout works after restart

---

## Test 6: Insufficient Vault Balance (Edge Case)
**Goal**: Verify graceful handling when Vault is empty

### Steps:
1. Use devtools to artificially drain Vault balance
   - OR: Test early in chain when Vault has minimal balance

2. Node wins mining without wallet â†’ rewards bank

3. Configure wallet

4. Check logs:
   ```
   [PENDING_REWARDS] âš ï¸ Failed to payout: Insufficient Vault balance (will retry)
   ```

5. Wait for retry (30s intervals)

6. When Vault accumulates enough balance, payout succeeds:
   ```
   [PENDING_REWARDS] ðŸŽ‰ Retry loop paid out...
   ```

**Expected Result**:
- âœ… Payout fails gracefully
- âœ… Retry loop continues attempting
- âœ… Eventually succeeds when Vault refills
- âœ… No loss of rewards

---

## Verification Checklist

### Logs to Check:
- [ ] `[PENDING_REWARDS] ðŸ’° Banked...` - Banking works
- [ ] `[PENDING_REWARDS] ðŸ’° Attempting payout...` - Payout triggered
- [ ] `[PENDING_REWARDS] âœ… Successfully paid out...` - Payout succeeded
- [ ] `[PENDING_REWARDS] ðŸŽ‰ Retry loop paid out...` - Retry works
- [ ] `[PENDING_REWARDS] âš ï¸ Failed to payout...` - Graceful failure

### Panel UI to Check:
- [ ] "Pending Rewards" field exists in Link Wallet section
- [ ] Shows "0 LAND" when no pending (grey color)
- [ ] Shows "X.XXXX LAND" when pending (yellow color)
- [ ] Updates after winning mining
- [ ] Updates to 0 after payout

### API to Check:
```powershell
# Get panel status
curl http://localhost:7070/api/panel/status | jq .pending_rewards

# Should return:
# 0 (if no pending)
# 1000000000000000000 (if 1 LAND pending)
```

### Database to Check:
```powershell
# List all pending rewards (requires DB inspection tool)
# Tree: "pending_rewards"
# Keys: node_id strings
# Values: u64 amounts
```

---

## Common Issues & Solutions

### Issue: Pending rewards not showing in UI
**Solution**: 
- Check `/api/panel/status` returns `pending_rewards` field
- Verify JavaScript console for errors
- Hard refresh browser (Ctrl+Shift+R)

### Issue: Payout not triggering
**Solution**:
- Verify wallet address is valid (not empty, not "Vault")
- Check logs for `try_payout_pending()` call
- Ensure Vault has sufficient balance
- Wait 30 seconds for retry loop

### Issue: Multiple payouts for same pending
**Solution**:
- Should NOT happen due to `pending_clear()`
- Check logs for duplicate payout messages
- Inspect sled DB to verify cleared

### Issue: Rewards lost after restart
**Solution**:
- Verify sled DB directory not deleted
- Check `pending_rewards` tree exists
- Run `pending_all()` to list all entries

---

## Performance Considerations

### Expected Load:
- **Banking**: Once per mining win (~every few minutes per node)
- **Payout**: Once when wallet configured + retry every 30s if pending
- **Storage**: O(1) reads/writes to sled DB

### No Performance Impact:
- âœ… No polling loops
- âœ… No unnecessary checks
- âœ… Efficient sled DB operations
- âœ… Direct balance transfers (no transaction overhead)

---

## Success Criteria

**System is working correctly if**:
1. âœ… Node without wallet banks rewards to Vault
2. âœ… Pending rewards persist across restarts
3. âœ… Configuring wallet triggers immediate payout
4. âœ… Retry loop pays out pending rewards automatically
5. âœ… UI displays pending amount accurately
6. âœ… Multiple wins accumulate correctly
7. âœ… Vaultâ†’User transfer is deterministic
8. âœ… No double-payouts occur
9. âœ… Insufficient balance fails gracefully
10. âœ… All nodes stay in consensus

---

## Quick Test Script (PowerShell)

```powershell
# Test 1: Check pending rewards via API
$status = Invoke-RestMethod "http://localhost:7070/api/panel/status"
Write-Host "Pending Rewards: $($status.pending_rewards / 1e18) LAND"

# Test 2: Check Vault balance
$vault = Invoke-RestMethod "http://localhost:7070/api/balance/Vault"
Write-Host "Vault Balance: $($vault / 1e18) LAND"

# Test 3: Check user balance
$user = "0x1234..."
$balance = Invoke-RestMethod "http://localhost:7070/api/balance/$user"
Write-Host "User Balance: $($balance / 1e18) LAND"

# Test 4: Watch logs for pending rewards
Get-Content vision-node.log -Wait | Select-String "PENDING_REWARDS"
```

---

## Next Steps After Testing

1. **If all tests pass**:
   - âœ… System is production-ready
   - âœ… Deploy to testnet
   - âœ… Monitor for 24-48 hours
   - âœ… Collect user feedback

2. **If issues found**:
   - Document the issue
   - Check logs for error messages
   - Review code at failure point
   - Add additional logging if needed
   - Re-test after fix

3. **Monitor in production**:
   - Watch for `[PENDING_REWARDS]` log patterns
   - Track Vault balance over time
   - Verify no consensus issues
   - Collect metrics on payout success rate

