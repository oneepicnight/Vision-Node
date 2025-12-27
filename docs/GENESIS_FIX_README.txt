===============================================================================
VISION NODE v2.1.0-CONSTELLATION - GENESIS BLOCK FIX
===============================================================================

IMPORTANT: BREAKING CHANGE - REQUIRES CHAIN RESET
===============================================================================

This release includes a CRITICAL fix to the genesis block implementation that
prevents nodes from forking at the start of the chain.

WHAT WAS FIXED:
---------------
Previously, each node independently created its own genesis block with slightly
different parameters (timestamps, random values). This caused immediate chain
forks at height 0, preventing network consensus.

Now all nodes use a deterministic canonical genesis block with hash:
af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262

REQUIRED ACTION FOR ALL OPERATORS:
-----------------------------------
1. STOP your existing Vision Node
2. BACKUP your chain.db file (optional, for safety)
3. DELETE the chain.db file from your data directory
4. START the new v2.1.0 node
5. Node will create correct genesis block automatically
6. Node will sync with network from genesis

WHAT TO EXPECT:
---------------
On startup, you will see:
  ✅ Genesis hash validation PASSED: af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262

If you see a VALIDATION FAILED message:
  ❌ GENESIS VALIDATION FAILED!
  
This means you have an old genesis block. Follow the steps above to delete
chain.db and restart.

WHY THIS MATTERS:
-----------------
Before this fix:
  - Each miner created different genesis blocks
  - Nodes forked immediately at height 0
  - No network consensus possible
  - Mining produced incompatible chains

After this fix:
  - All nodes start with identical genesis block
  - Network reaches consensus from the start
  - Mining builds on shared chain history
  - Proper blockchain operation restored

GENESIS BLOCK DETAILS:
----------------------
Version:     1
Height:      0
Prev Hash:   0000...0000 (64 zeros)
Timestamp:   0 (Unix epoch)
Difficulty:  1
Nonce:       0
TX Root:     0000...0000 (64 zeros)
POW Hash:    af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262

The POW hash is computed deterministically using Blake3:
  Blake3(version || height || prev_hash || timestamp || difficulty || nonce || tx_root)

NETWORK COORDINATION:
---------------------
This is a coordinated network upgrade. All operators must:
1. Upgrade to v2.1.0-CONSTELLATION
2. Delete old chain.db files
3. Restart nodes together
4. Network will sync from common genesis

TESTING BEFORE DEPLOYMENT:
--------------------------
To test the genesis fix:
1. Delete chain.db
2. Start node
3. Check logs for "✅ Genesis hash validation PASSED"
4. Verify genesis hash matches: af1349b9f5...
5. Start second node with fresh chain.db
6. Verify both nodes create identical genesis
7. Verify nodes can sync and build consensus

TROUBLESHOOTING:
----------------
Q: Node panics on startup with genesis validation error
A: Delete chain.db and restart - you have an old genesis block

Q: Can I keep my old blockchain data?
A: No - old data is incompatible. You must start fresh with correct genesis.

Q: Will I lose my wallet balance?
A: Balances are stored separately from blockchain (in database). However,
   since this is a breaking network reset, balances will reset to genesis
   state. This is expected for testnet operation.

Q: What if I skip this upgrade?
A: Your node will be unable to sync with the network. All nodes must upgrade
   together for consensus to work.

ADDITIONAL IMPROVEMENTS IN v2.1.0:
----------------------------------
✅ Genesis block deterministic hash (CRITICAL FIX)
✅ P2P port resolution consistency (HTTP=7070, P2P=7072)
✅ Gossip protocol integration for automatic peer discovery
✅ Connection tracking improvements (await results, not fire-and-forget)
✅ Continuous discovery until min_peer_store_population reached
✅ Retry counter reset on valid advertised addresses
✅ Dual-target connection maintenance (connections + peer store)

SUPPORT:
--------
For questions or issues with the genesis fix:
- Check logs for genesis validation messages
- Verify chain.db is deleted before restart
- Confirm genesis hash matches canonical: af1349b9f5...
- Contact network administrators if problems persist

Version: v2.1.0-CONSTELLATION
Build Date: 2025
Genesis Hash: af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262
