# Vision Node – MVP Endpoints (Frozen)

**Last Updated:** October 31, 2025  
**Status:** Production-Ready for Genesis Launch  
**Build:** `cargo build --release` (FULL only)

---

## Philosophy

This document defines the **frozen MVP API surface** — the essential endpoints needed to launch Vision World and support the initial community of miners, landowners, and players.

The node is shipped as a **FULL-only** build, but the MVP surface remains the stable, production-facing contract.

**What's Included:**
- ✅ Blockchain fundamentals (blocks, consensus, P2P)
- ✅ Wallet system (balances, transfers, receipts)
- ✅ Market economy (LAND/CASH trades, 50/30/20 splits)
- ✅ Vault epochs (land staking rewards)
- ✅ Admin tools (development and testing)
- ✅ Observability (metrics, health checks)

**Not Covered Here (Experimental / Non-MVP):**
- ⏸️ Zero-knowledge proofs
- ⏸️ Cross-chain bridges
- ⏸️ Sharding
- ⏸️ Advanced governance
- ⏸️ Experimental VMs

---

## Core / Chain

### `GET /livez`
**Purpose:** Liveness probe (K8s/Docker health check)  
**Response:** `200 OK` with `"ok"` text  
**Auth:** None

### `GET /readyz`
**Purpose:** Readiness probe (is node ready to accept requests?)  
**Response:** `200 OK` with `{"ready": true}`  
**Auth:** None

### `GET /health/score`
**Purpose:** Overall health score (0-100)  
**Response:** `{"score": 95, "details": {...}}`  
**Auth:** None

### `GET /config`
**Purpose:** Node configuration (epoch length, fees, limits)  
**Response:** `{"epoch_blocks": 180, "fee_base": 1000, ...}`  
**Auth:** None

### `GET /height`
**Purpose:** Current chain height  
**Response:** `{"height": 12345}`  
**Auth:** None

### `GET /status`
**Purpose:** Node status with sync progress and network health  
**Response:**
```json
{
  "height": 12345,
  "best_peer_height": 12346,
  "lag": 1,
  "mempool": 15,
  "mining_allowed": true,
  "gating": true,
  "max_lag": 10,
  "peers": ["http://peer1.com:7070"],
  "difficulty": 1000,
  "target_block_time": 60,
  "p2p_peers": [
    {
      "address": "192.168.1.100:7071",
      "peer_id": "peer-1234567890abcdef",
      "height": 12345,
      "direction": "outbound",
      "last_activity_secs": 5
    }
  ],
  "p2p_peer_count": 3,
  "p2p_inbound_count": 1,
  "p2p_outbound_count": 2,
  "network_health_score": 85.0
}
```
**Auth:** None

### `GET /tcp_peers`
**Purpose:** Active TCP P2P peer connections  
**Response:** `{"peers": [...], "count": 3}`  
**Auth:** None

---

## Peers (P2P Networking)

### `GET /peers/list`
**Purpose:** List connected peers  
**Response:** `{"peers": ["http://peer1.com:7070", "http://peer2.com:7070"]}`  
**Auth:** None

### `POST /peers/add`
**Purpose:** Add peer to network  
**Body:** `{"url": "http://newpeer.com:7070"}`  
**Response:** `{"ok": true}`  
**Auth:** Admin token (dev-only)

### `POST /peers/remove`
**Purpose:** Remove peer from network  
**Body:** `{"url": "http://badpeer.com:7070"}`  
**Response:** `{"ok": true}`  
**Auth:** Admin token (dev-only)

---

## Snapshots (Backup/Restore)

### `POST /admin/backup`
**Purpose:** Create blockchain snapshot (backup current state)  
**Response:** `{"ok": true, "path": "snapshot-20251031.tar.gz"}`  
**Auth:** Admin token

### `POST /admin/restore`
**Purpose:** Restore from snapshot  
**Body:** `{"snapshot_path": "snapshot-20251031.tar.gz"}`  
**Response:** `{"ok": true, "restored_height": 12340}`  
**Auth:** Admin token

---

## Wallet & Transfers

### `GET /wallet/:addr/balance`
**Purpose:** Query token balance for address  
**Params:** `:addr` - Address (min 8 chars)  
**Response:** `{"address": "alice12345678", "balance": "1000000"}`  
**Auth:** None

### `POST /wallet/transfer`
**Purpose:** Transfer tokens between addresses  
**Body:**
```json
{
  "from": "alice12345678",
  "to": "bob987654321",
  "amount": "5000",
  "fee": "50",
  "memo": "Payment for land"
}
```
**Response:** `{"status": "ok", "receipt_id": "latest"}`  
**Auth:** None (signature verification in future)

---

## Admin (Development Tools)

### `POST /admin/seed-balance`
**Purpose:** Seed initial balances for testing (dev-only)  
**Body:**
```json
{
  "address": "testuser123456",
  "amount": "1000000"
}
```
**Response:** `{"ok": true, "new_balance": "1000000"}`  
**Auth:** Admin token (`VISION_ADMIN_TOKEN`)  
**Flags:** Requires `VISION_DEV=1` environment variable

---

## Market & Vault

### `POST /market/list`
**Purpose:** List item for sale (LAND or CASH)  
**Body:**
```json
{
  "seller": "alice12345678",
  "item_type": "LAND",
  "item_id": "parcel_001",
  "price": "50000",
  "currency": "CASH"
}
```
**Response:** `{"listing_id": "listing_12345"}`  
**Auth:** None

### `POST /market/buy`
**Purpose:** Buy listed item (triggers 50/30/20 split)  
**Body:**
```json
{
  "buyer": "bob987654321",
  "listing_id": "listing_12345"
}
```
**Response:** `{"ok": true, "settlement": {...}}`  
**Auth:** None

### `GET /vault/info`
**Purpose:** Vault statistics (total balance, splits, recent events)  
**Response:**
```json
{
  "vault_balance": "5000000",
  "ops_balance": "3000000",
  "founders_balance": "2000000",
  "last_10_events": [...]
}
```
**Auth:** None

### `GET /vault/epoch`
**Purpose:** Epoch payout status  
**Response:**
```json
{
  "epoch_index": 42,
  "last_payout_height": 1260,
  "next_payout_height": 1440,
  "vault_balance": "500000",
  "total_weight": "25",
  "due": false
}
```
**Auth:** None

---

## Observability

### `GET /metrics`
**Purpose:** Prometheus metrics (Grafana integration)  
**Response:** Text format Prometheus metrics  
**Example:**
```
# HELP vision_blocks_height Current blockchain height
# TYPE vision_blocks_height gauge
vision_blocks_height 12345
```
**Auth:** None

---

## What's NOT in MVP

These endpoints require `--features full` (advanced build):

### Zero-Knowledge
- `/zk/proof/generate`
- `/zk/verify`
- `/zk/circuits`

### Cross-Chain Bridges
- `/bridge/lock`
- `/bridge/unlock`
- `/bridge/relay`
- `/ibc/clients`
- `/ibc/channels`
- `/ibc/transfer`

### Sharding
- `/shard/info/:id`
- `/shard/assign`
- `/shard/crosslink`

### Advanced Governance
- `/gov/proposal/create`
- `/gov/vote`
- `/gov/tally/:id`

### Smart Contracts (Multi-VM)
- `/vm/evm/deploy`
- `/vm/evm/call`
- `/vm/cross-call`
- `/contract/deploy`
- `/contract/call`

### State Channels
- `/channel/open`
- `/channel/update`
- `/channel/close`

### Data Availability Layer
- `/da/blob/submit`
- `/da/sample/:blob_id`

### MEV Protection
- `/bundle/submit`
- `/bundle/status/:id`

### Oracle Networks
- `/oracle/register`
- `/oracle/price/:feed_id`

### DID (Decentralized Identity)
- `/did/register`
- `/did/:id`

### Account Abstraction
- `/account/abstract/create`
- `/account/abstract/execute`

### Hardware Wallet Support
- `/wallet/devices`
- `/wallet/sign`

### IPFS Integration
- `/ipfs/upload`
- `/ipfs/:cid`

### HTLC (Atomic Swaps)
- `/htlc/create`
- `/htlc/:id/claim`

### Advanced Analytics
- `/analytics/flow`
- `/analytics/clusters`
- `/analytics/graph`

### Light Client Support
- `/light/sync`
- `/light/verify/tx`

### Block Explorer (Advanced)
- `/explorer/trace/:tx_hash`
- `/explorer/contract/:address/code`

---

## Enforcement

### PR Checklist
- [ ] MVP endpoint: documented in this file
- [ ] Route registered in the main router
- [ ] Integration test added/updated
- [ ] Maintainer approval for MVP surface changes

---

## Build

```bash
cargo build --release
```

---

## Rationale

**Why freeze the MVP surface?**

1. **Focus** — Launch with 20 rock-solid endpoints, not 200 half-baked ones
2. **Testing** — 100% coverage on MVP features
3. **Security** — Reduced exposure on the stable surface
5. **Documentation** — Every MVP endpoint fully documented

MVP surface expansions require explicit maintainer approval and a documented rollout plan.

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-10-31 | Initial MVP surface frozen |

---

**Vision Node MVP** — *Built to last, not to impress.*
