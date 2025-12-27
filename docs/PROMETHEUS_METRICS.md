# Vision Node Prometheus Metrics

## Overview

The Vision node exposes Prometheus-compatible metrics at `/metrics` for monitoring tokenomics, chain health, and operational status.

## Endpoint

```
GET /metrics
```

**Content-Type:** `text/plain; version=0.0.4; charset=utf-8`

## Available Metrics

### Tokenomics Metrics

All tokenomics metrics are gauges refreshed from the database on each scrape.

| Metric | Type | Description |
|--------|------|-------------|
| `vision_tok_supply` | Gauge | Current total token supply |
| `vision_tok_burned_total` | Gauge | Cumulative burned tokens |
| `vision_tok_vault_total` | Gauge | Vault (staking) balance |
| `vision_tok_fund_total` | Gauge | Ecosystem/Fund balance |
| `vision_tok_treasury_total` | Gauge | Treasury (founders) balance |

### Operational Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `vision_blocks_height` | Gauge | Current best chain height |
| `vision_mempool_len` | Gauge | Current mempool size (critical + bulk) |
| `vision_peers_connected` | Gauge | Number of connected peers |

## Database Schema

Tokenomics metrics are read from the `tokenomics` tree in sled:

```
Tree: "tokenomics"
  Key: "supply"           → u128 (little-endian)
  Key: "burned_total"     → u128 (little-endian)
  Key: "vault_total"      → u128 (little-endian)
  Key: "fund_total"       → u128 (little-endian)
  Key: "treasury_total"   → u128 (little-endian)
```

**Note:** If keys are missing, they default to 0.

## Example Response

```prometheus
# HELP vision_tok_supply Current total token supply
# TYPE vision_tok_supply gauge
vision_tok_supply 1000000000

# HELP vision_tok_burned_total Cumulative burned tokens
# TYPE vision_tok_burned_total gauge
vision_tok_burned_total 5000000

# HELP vision_tok_vault_total Vault (staking) balance
# TYPE vision_tok_vault_total gauge
vision_tok_vault_total 500000000

# HELP vision_tok_fund_total Ecosystem/Fund balance
# TYPE vision_tok_fund_total gauge
vision_tok_fund_total 300000000

# HELP vision_tok_treasury_total Treasury (founders) balance
# TYPE vision_tok_treasury_total gauge
vision_tok_treasury_total 195000000

# HELP vision_blocks_height Best chain height
# TYPE vision_blocks_height gauge
vision_blocks_height 1234

# HELP vision_mempool_len Current mempool length
# TYPE vision_mempool_len gauge
vision_mempool_len 42

# HELP vision_peers_connected Connected peers
# TYPE vision_peers_connected gauge
vision_peers_connected 5
```

## Integration with Prometheus

### prometheus.yml Configuration

```yaml
scrape_configs:
  - job_name: 'vision-node'
    scrape_interval: 15s
    static_configs:
      - targets: ['127.0.0.1:7070']
    metrics_path: /metrics
```

### Docker Compose Example

```yaml
version: '3.8'
services:
  vision-node:
    image: vision-node:latest
    ports:
      - "7070:7070"
    environment:
      - VISION_PORT=7070
      - VISION_ADMIN_TOKEN=your-secret-token

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
```

## Grafana Dashboard

### Sample Queries

**Total Token Supply:**
```promql
vision_tok_supply
```

**Token Distribution Pie Chart:**
```promql
vision_tok_vault_total + on() group_left() vector(0) * (vision_tok_supply - vision_tok_vault_total)
vision_tok_fund_total + on() group_left() vector(0) * (vision_tok_supply - vision_tok_fund_total)
vision_tok_treasury_total + on() group_left() vector(0) * (vision_tok_supply - vision_tok_treasury_total)
```

**Block Production Rate (blocks/minute):**
```promql
rate(vision_blocks_height[5m]) * 60
```

**Mempool Growth Rate:**
```promql
rate(vision_mempool_len[5m])
```

**Peer Connectivity:**
```promql
vision_peers_connected
```

### Sample Dashboard Panels

#### Token Distribution Panel
```json
{
  "title": "Token Distribution",
  "type": "piechart",
  "targets": [
    {
      "expr": "vision_tok_vault_total",
      "legendFormat": "Vault"
    },
    {
      "expr": "vision_tok_fund_total",
      "legendFormat": "Fund"
    },
    {
      "expr": "vision_tok_treasury_total",
      "legendFormat": "Treasury"
    },
    {
      "expr": "vision_tok_burned_total",
      "legendFormat": "Burned"
    }
  ]
}
```

#### Chain Health Panel
```json
{
  "title": "Chain Health",
  "type": "stat",
  "targets": [
    {
      "expr": "vision_blocks_height",
      "legendFormat": "Height"
    },
    {
      "expr": "vision_mempool_len",
      "legendFormat": "Mempool"
    },
    {
      "expr": "vision_peers_connected",
      "legendFormat": "Peers"
    }
  ]
}
```

## Testing

### Manual Test

```bash
# Start the node
cargo run --release

# In another terminal, fetch metrics
curl http://127.0.0.1:7070/metrics
```

### PowerShell Test

```powershell
# Fetch and display metrics
Invoke-RestMethod -Uri "http://127.0.0.1:7070/metrics"

# Filter for tokenomics metrics only
(Invoke-RestMethod -Uri "http://127.0.0.1:7070/metrics") -split "`n" | Where-Object { $_ -match "vision_tok" }
```

### Verify Prometheus Scraping

```bash
# Check if Prometheus is scraping the target
curl http://localhost:9090/api/v1/targets

# Query a specific metric
curl 'http://localhost:9090/api/v1/query?query=vision_tok_supply'
```

## Updating Metrics at Runtime

The metrics module provides helper methods to update operational metrics from your code:

```rust
// Get the metrics handle
let metrics = PROM_METRICS.clone();

// Update when a block is mined
metrics.set_height(new_height);

// Update after mempool changes
metrics.set_mempool_len(mempool.len());

// Update after peer connect/disconnect
metrics.set_peers(peer_count);
```

Tokenomics metrics are automatically refreshed from the database on each `/metrics` request.

## Populating Tokenomics Data

To populate the tokenomics tree in sled for testing:

```rust
// Open the tokenomics tree
let db = sled::open("./vision_data_7070")?;
let tree = db.open_tree("tokenomics")?;

// Store sample values (u128 little-endian)
tree.insert(b"supply", &1_000_000_000u128.to_le_bytes())?;
tree.insert(b"burned_total", &5_000_000u128.to_le_bytes())?;
tree.insert(b"vault_total", &500_000_000u128.to_le_bytes())?;
tree.insert(b"fund_total", &300_000_000u128.to_le_bytes())?;
tree.insert(b"treasury_total", &195_000_000u128.to_le_bytes())?;

db.flush()?;
```

Or via a PowerShell script:

```powershell
# See test-metrics-population.ps1 for a complete example
```

## Alerting Rules

### Example Prometheus Alerts

```yaml
groups:
  - name: vision_node
    rules:
      - alert: VisionNodeDown
        expr: up{job="vision-node"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Vision node is down"

      - alert: LowPeerCount
        expr: vision_peers_connected < 2
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Low peer count ({{ $value }})"

      - alert: MempoolClogged
        expr: vision_mempool_len > 5000
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Mempool is clogged ({{ $value }} txs)"

      - alert: TokenSupplyAnomaly
        expr: increase(vision_tok_supply[1h]) > 1000000
        for: 1h
        labels:
          severity: critical
        annotations:
          summary: "Abnormal token supply increase"
```

## Troubleshooting

### Metrics Return Empty or Zeros

**Problem:** All tokenomics metrics show 0.

**Solution:** The `tokenomics` tree in sled is empty. Populate it with initial values or ensure your settlement/minting logic writes to this tree.

### Metrics Endpoint Returns 500

**Problem:** Database read error or encoding failure.

**Solution:** Check logs for specific error. The handler is forgiving and will render existing gauge values even if DB refresh fails.

### Prometheus Not Scraping

**Problem:** Prometheus shows target as "DOWN".

**Solution:**
- Verify node is running: `curl http://127.0.0.1:7070/metrics`
- Check firewall rules
- Verify prometheus.yml has correct target address
- Check Prometheus logs

## See Also

- `src/metrics.rs` - Metrics module implementation
- `src/main.rs` - Handler integration
- `test-metrics.ps1` - Test script
- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)
