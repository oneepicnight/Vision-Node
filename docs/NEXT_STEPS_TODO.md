# Vision Node - Next Steps TODO
Based on v0.7.9 Release Notes Future Roadmap

## 🎯 v0.8.0 Features (Priority)

### 1. Dynamic Fee Estimation ⏳
- [ ] Implement `estimatesmartfee` RPC integration
- [ ] Add fee tier selection (economy, normal, priority)
- [ ] Cache fee estimates with 10-minute TTL
- [ ] Add `/api/fees/estimate` endpoint
- [ ] Update tx_builder to use dynamic fees instead of fixed
- [ ] Add fee preview in wallet UI before sending
- [ ] Document fee estimation algorithm

**Files to modify:**
- `src/tx_builder.rs` - Replace fixed fees with dynamic
- `src/send.rs` - Add fee estimation step
- New: `src/fee_estimator.rs` - Fee estimation module

### 2. UTXO Background Sync Task ⏳
- [ ] Create async background task using tokio
- [ ] Periodic UTXO sync (every 30 seconds)
- [ ] Sync on new block notifications
- [ ] Track UTXO confirmations
- [ ] Handle reorgs and invalidated UTXOs
- [ ] Add UTXO cache with expiration
- [ ] Log sync status and errors

**Files to modify:**
- `src/utxo_manager.rs` - Add background sync
- `src/main.rs` - Spawn UTXO sync task on startup
- Add tokio task management

### 3. Transaction History & Confirmation Tracking ⏳
- [ ] Create transaction history table/storage
- [ ] Track sent transactions with timestamps
- [ ] Monitor transaction confirmations via RPC
- [ ] Add `/api/wallet/transactions` endpoint
- [ ] Display transaction status (pending, confirmed, failed)
- [ ] Add confirmation count to UI
- [ ] Filter by date range and status
- [ ] Export transaction history to CSV

**Files to create:**
- `src/tx_history.rs` - Transaction history module
- Database schema for tx history

**API endpoints:**
- `GET /api/wallet/transactions` - List all transactions
- `GET /api/wallet/transaction/:txid` - Get transaction details
- `GET /api/wallet/transactions/pending` - Pending transactions only

### 4. Multi-Signature Wallet Support 🔐
- [ ] P2SH address generation for multisig
- [ ] N-of-M signature collection flow
- [ ] Partial signature storage
- [ ] Signature aggregation
- [ ] Multisig transaction broadcasting
- [ ] Add multisig UI in wallet
- [ ] Support for 2-of-3, 3-of-5 schemes

**Files to create:**
- `src/multisig.rs` - Multisig wallet module
- `src/multisig_ui/` - UI components

### 5. WebSocket Notifications for Transaction Status 🔔
- [ ] Add WebSocket endpoint `/ws/wallet`
- [ ] Real-time transaction status updates
- [ ] Balance change notifications
- [ ] UTXO updates via WebSocket
- [ ] Connection management and reconnection
- [ ] Add to wallet UI for live updates
- [ ] Rate limiting for WebSocket messages

**Files to modify:**
- `src/main.rs` - Add WebSocket route
- New: `src/websocket/wallet.rs` - Wallet WebSocket handler
- Update wallet UI to consume WebSocket events

---

## 🚀 v0.9.0 Features (Future)

### 1. Lightning Network Integration ⚡
- [ ] LND or CLN node integration
- [ ] Open/close channels
- [ ] Lightning invoice creation
- [ ] Lightning payment sending
- [ ] Channel balance management
- [ ] Routing node configuration
- [ ] Lightning Network UI dashboard

**Dependencies:**
- `tonic-lnd` or equivalent for LND
- gRPC client setup

### 2. Atomic Swaps Between Chains 🔄
- [ ] HTLC (Hash Time-Locked Contract) implementation
- [ ] Cross-chain swap protocol
- [ ] Swap order matching
- [ ] Timelock management
- [ ] Refund handling on failure
- [ ] Swap UI with price discovery
- [ ] Support BTC ↔ BCH, BTC ↔ DOGE

**Files to create:**
- `src/atomic_swap/` - Atomic swap module
- `src/atomic_swap/htlc.rs` - HTLC logic
- `src/atomic_swap/protocol.rs` - Swap protocol

### 3. Advanced UTXO Coin Selection Algorithms 🎯
- [ ] Branch and Bound algorithm
- [ ] Knapsack solver
- [ ] Privacy-preserving coin selection
- [ ] Minimize fees with optimal UTXO selection
- [ ] Avoid address reuse
- [ ] UTXO consolidation strategies
- [ ] A/B testing different algorithms

**Files to modify:**
- `src/utxo_manager.rs` - Add new selection algorithms
- Add `utxo_selection/` module with multiple strategies

**Algorithms to implement:**
- Largest-first (current)
- Branch and Bound (Bitcoin Core default)
- Knapsack
- Random selection
- Privacy-focused selection

### 4. Hardware Wallet Integration 🔐
- [ ] Ledger device support
- [ ] Trezor device support
- [ ] USB HID communication
- [ ] Transaction signing flow with hardware
- [ ] Address derivation from hardware
- [ ] BIP39/BIP44 HD wallet paths
- [ ] Hardware wallet UI

**Dependencies:**
- `ledger-transport` for Ledger
- `trezor-client` for Trezor

---

## 🔧 Infrastructure Improvements

### Testing & Quality
- [ ] Integration tests for send endpoint
- [ ] UTXO manager unit tests
- [ ] Transaction builder test suite
- [ ] Mock RPC server for testing
- [ ] End-to-end wallet tests
- [ ] Load testing for UTXO sync

### Security
- [ ] Audit key_manager.rs encryption
- [ ] Implement proper key derivation (PBKDF2)
- [ ] Add rate limiting to send endpoint
- [ ] Transaction amount limits
- [ ] IP-based rate limiting
- [ ] Audit trail for all sends

### Monitoring
- [ ] Prometheus metrics for tx success/failure
- [ ] UTXO sync lag monitoring
- [ ] RPC connection health checks
- [ ] Alert on transaction failures
- [ ] Dashboard for wallet operations

---

## 📝 Documentation Needed

### User Documentation
- [ ] Complete wallet user guide
- [ ] How to send crypto step-by-step
- [ ] Fee estimation explanation
- [ ] Transaction confirmation guide
- [ ] Multisig wallet setup tutorial

### Developer Documentation
- [ ] API reference for all wallet endpoints
- [ ] UTXO manager architecture docs
- [ ] Transaction builder deep dive
- [ ] WebSocket protocol specification
- [ ] Contributing guide for wallet features

---

## 🎯 Pool System Enhancements

### Already Complete ✅
- ✅ Worker mining loop (JoinPool mode)
- ✅ Transaction-based payout architecture
- ✅ Integration tests
- ✅ Performance hardening (rate limiting, caching, banning, metrics)
- ✅ Pool API endpoints (register, job, share, stats, metrics, configure, start, stop)
- ✅ Worker names for joiners
- ✅ Host can see all worker names
- ✅ Pool URL persistence and restart handling
- ✅ AAA-grade UI for pool panel

### Pool System TODO
- [ ] Stratum protocol support (for mining pool compatibility)
- [ ] Pool discovery service (public pool directory)
- [ ] Variable difficulty per worker
- [ ] Long polling for job updates (reduce network traffic)
- [ ] Pool failover support (backup pools)
- [ ] Pool analytics dashboard (revenue, efficiency)
- [ ] Payout scheduling (hourly, daily, threshold-based)
- [ ] Email notifications for pool operators

---

## 🔥 High Priority Items

1. **Dynamic Fee Estimation** - Users need accurate fees
2. **UTXO Background Sync** - Prevent stale UTXO issues
3. **Transaction History** - Users need to see past transactions
4. **WebSocket Notifications** - Better UX with real-time updates
5. **Security Audit** - Key manager and send endpoint

## 📅 Timeline Estimate

- **v0.8.0:** 4-6 weeks (Jan 2026)
- **v0.9.0:** 8-10 weeks (Mar 2026)

---

**Status:** 🚧 In Planning
**Last Updated:** November 21, 2025
