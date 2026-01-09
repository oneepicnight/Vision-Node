# VAULT HARDENING QUICK REFERENCE

**Status**: ✅ Production Ready  
**Version**: v1.0.0 mainnet  
**Priority**: CRITICAL

---

## What Changed?

1. **Economics Fingerprint**: All vault addresses + reward splits now locked via Blake3 hash
2. **Startup Validation**: Node exits if local addresses don't match canonical hash  
3. **P2P Enforcement**: Peers with wrong addresses are rejected during handshake
4. **No Hardcoded Addresses**: All vault addresses loaded from `config/token_accounts.toml`

---

## Canonical Vault Addresses

**ECON_HASH**: `a18f9f82aeb6276b5cfb353e351cd0cf9b34aad962e29f4ac6268f0659c55f95`

**Addresses** (DO NOT MODIFY):
```
Staking Vault (50%):   0xb977c16e539670ddfecc0ac902fcb916ec4b944e
Ecosystem Fund (30%):  0x8bb8edcd4cdbcb132cc5e88ff90ba48cebf11cbd
Founder1 (10%):        0xdf7a79291bb96e9dd1c77da089933767999eabf0
Founder2 (10%):        0x083f95edd48e3e9da396891b704994b86e7790e7
```

---

## Key Commands

### Check Econ Hash
```bash
curl http://localhost:7070/status | jq .econ_hash
# Should output: "a18f9f82aeb6276b5cfb353e351cd0cf9b34aad962e29f4ac6268f0659c55f95"
```

### Verify Config
```bash
cat config/token_accounts.toml | grep address
# Should show the 4 addresses above
```

### Recompute Hash (if needed)
```bash
cargo test --bin vision-node test_mainnet_econ_hash -- --nocapture
# Prints canonical hash
```

---

## Startup Behavior

### ✅ SUCCESS
```
Token accounts loaded: vault=0xb977..., fund=0x8bb8..., ...
🔒 Validating economics fingerprint (vault addresses + splits)...
✅ Economics fingerprint validation PASSED: a18f9f...
```

### ❌ FAILURE
```
❌ CRITICAL FAILURE: Economics validation failed!
CRITICAL: Economics fingerprint mismatch!
Expected (canonical): a18f9f82aeb6276b5cfb353e351cd0cf9b34aad962e29f4ac6268f0659c55f95
Computed: <wrong hash>
...
🛑 Node startup ABORTED.
```

**Fix**: Restore `config/token_accounts.toml` from official source

---

## P2P Handshake

### What Gets Validated
- `genesis_hash` - Block 0 hash (existing)
- `chain_id` - Network identifier (existing)
- **`econ_hash`** - Vault addresses + splits (NEW)

### Rejection Behavior
```
❌ HANDSHAKE REJECT: ECON_HASH mismatch. expected=a18f9f... got=<wrong>
Vault address tampering detected - REJECTING CONNECTION.
```

**Result**: Peer is not added to routing table, connection closes immediately

---

## Troubleshooting

### "Node won't start - economics validation failed"
**Cause**: Your `config/token_accounts.toml` has wrong addresses or splits  
**Fix**: `git checkout config/token_accounts.toml` or restore from backup

### "Can't connect to peers - handshake rejected"
**Cause**: Your node or peer has mismatched econ_hash  
**Fix**: Ensure all nodes use identical `token_accounts.toml`

### "How do I change vault addresses?"
**Answer**: You DON'T - requires network-wide hard fork. Contact core team.

---

## Files Modified

**New Files**:
- `src/chain/economics.rs` - Economics fingerprint module

**Modified Files**:
- `src/genesis.rs` - Added ECON_HASH constant + validation functions
- `src/main.rs` - Added startup validation + econ_hash to status endpoint
- `src/p2p/connection.rs` - Added econ_hash to handshake + validation
- `config/token_accounts.toml` - Real mainnet addresses
- `src/vision_constants.rs` - Removed legacy VAULT_ADDRESS constants
- `src/pending_rewards.rs` - Use vault_address() function
- `src/pool/payouts.rs` - Use vault_address() function
- `src/pool/state.rs` - Use vault_address() function
- `src/tokenomics/tithe.rs` - Use founder_address() and ops_address() functions

---

## Security Guarantees

**✅ Tamper Detection**: Any modification to vault addresses → Node exits  
**✅ Fork Isolation**: Nodes with different addresses → Cannot connect  
**✅ Consensus Lock**: All nodes validate identical economics fingerprint  
**✅ Audit Trail**: All mismatches logged with clear error messages

---

## Quick Tests

### 1. Verify Startup
```bash
./target/release/vision-node 2>&1 | grep -A5 "Economics fingerprint"
# Should see: "✅ Economics fingerprint validation PASSED"
```

### 2. Verify Status Endpoint
```bash
curl -s http://localhost:7070/status | jq '{econ_hash, height, peers: .peers | length}'
# Should show econ_hash + current stats
```

### 3. Verify P2P Connections
```bash
curl -s http://localhost:7070/status | jq '.peers'
# Should list connected peers (proves handshake succeeded)
```

---

## Emergency Contacts

**If node won't start**:
1. Check logs: `tail -f vision-node.log | grep CRITICAL`
2. Verify config: `sha256sum config/token_accounts.toml`
3. Restore backup: `cp config/token_accounts.toml.backup config/token_accounts.toml`
4. Contact support if issue persists

**If P2P rejections spike**:
1. Check peer versions: `curl http://peer:7070/status`
2. Verify all nodes updated to latest build
3. Monitor: `grep "ECON_HASH mismatch" p2p.log`

---

## Deployment Status

- [x] Code complete
- [x] Build successful (warnings only, no errors)
- [x] Unit tests pass
- [ ] Deployed to mainnet nodes (pending)
- [ ] All nodes report matching econ_hash (pending)
- [ ] P2P connections stable (pending)

---

**For Full Details**: See `MAINNET_VAULT_HARDENING_COMPLETE.md`
