# Tokenomics Quick Test Guide

## Overview
Vision Node implements a complete tokenomics system with:
- **Block emission** with Bitcoin-style halving (~4 years)
- **Fee distribution** (10% of transaction fees go to 50/30/20 split: Vault/Fund/Treasury)
- **Treasury siphon** (5% of emission to governance treasury)
- **Land sale splits** (50% Vault, 30% Fund, 20% Treasury)
- **Staking epoch payouts** (pro-rata from Vault every N blocks)

**IMPORTANT**: There is NO coin burning. The `fee_burn_bps` parameter distributes fees to governance funds (50% Vault, 30% Fund, 20% Treasury), not destroys them.

## Quick Start

### 1. Check Tokenomics Stats
```bash
# Get current tokenomics configuration and state
curl -s http://127.0.0.1:7070/tokenomics/stats | jq

# Expected output:
# {
#   "ok": true,
#   "config": {
#     "enable_emission": true,
#     "emission_per_block": "1000000000000",
#     "halving_interval_blocks": 2102400,
#     "fee_burn_bps": 1000,
#     "treasury_bps": 500,
#     "staking_epoch_blocks": 720,
#     "vault_addr": "...",
#     "fund_addr": "...",
#     "treasury_addr": "..."
#   },
#   "state": {
#     "current_height": 42,
#     "total_supply": "50000000000000",
#     "burned": "1200000000",
#     "treasury_total": "2500000000",
#     "vault_total": "0",
#     "fund_total": "0",
#     "next_halving_height": 2102400
#   }
# }
```

### 2. Preview Emission at Different Heights
```bash
# Check emission at genesis
curl -s http://127.0.0.1:7070/tokenomics/emission/0 | jq

# Check emission at first halving (block 2,102,400)
curl -s http://127.0.0.1:7070/tokenomics/emission/2102400 | jq

# Check emission after 3 halvings
curl -s http://127.0.0.1:7070/tokenomics/emission/6307200 | jq

# Loop through heights
for h in 0 100 1000 10000 1000000 2102400 4204800; do 
  echo "Height $h:"
  curl -s http://127.0.0.1:7070/tokenomics/emission/$h | jq -c
  echo
done
```

### 3. Mine Blocks (Tokenomics Auto-Applied)
```bash
# Mine a block - emission, burning, treasury automatically applied
curl -X POST http://127.0.0.1:7070/mine \
  -H "Content-Type: application/json" \
  -d '{"miner_addr":"miner1","max_txs":100}'

# Check updated stats
curl -s http://127.0.0.1:7070/tokenomics/stats | jq .state
```

### 4. Staking Operations

#### Stake Tokens
```bash
# Stake 1000 tokens
curl -X POST http://127.0.0.1:7070/staking/stake \
  -H "Content-Type: application/json" \
  -d '{
    "staker": "alice",
    "amount": 1000000000000
  }' | jq

# Expected: {"ok":true,"staker":"alice","staked_amount":1000000000000,"total_staked":"1000000000000"}
```

#### Check Staking Info
```bash
# Get staker details
curl -s http://127.0.0.1:7070/staking/info/alice | jq

# Get overall staking statistics
curl -s http://127.0.0.1:7070/staking/stats | jq

# Expected output:
# {
#   "ok": true,
#   "total_staked": "5000000000000",
#   "stakers_count": 3,
#   "last_epoch_height": 0,
#   "current_height": 150,
#   "epoch_interval": 720,
#   "blocks_until_next_epoch": 570,
#   "vault_balance": "10000000000000"
# }
```

#### Unstake Tokens
```bash
# Unstake 500 tokens
curl -X POST http://127.0.0.1:7070/staking/unstake \
  -H "Content-Type: application/json" \
  -d '{
    "staker": "alice",
    "amount": 500000000000
  }' | jq
```

### 5. Watch Staking Epoch Payouts

```bash
# Mine blocks until epoch boundary (every 720 blocks by default)
for i in {1..800}; do
  curl -s -X POST http://127.0.0.1:7070/mine \
    -H "Content-Type: application/json" \
    -d '{"miner_addr":"miner1"}' > /dev/null
  
  # Check every 100 blocks
  if [ $((i % 100)) -eq 0 ]; then
    echo "Block $i:"
    curl -s http://127.0.0.1:7070/staking/stats | jq -c \
      '{height: .current_height, blocks_until_epoch: .blocks_until_next_epoch}'
  fi
done

# Check logs for staking payout event at block 720
```

### 6. Admin: Tune Tokenomics Config (Requires Admin Token)

```bash
# Update fee burn percentage to 5% (500 bps)
curl -X POST "http://127.0.0.1:7070/admin/tokenomics/config?admin_token=YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "fee_burn_bps": 500,
    "treasury_bps": 1000
  }' | jq

# Disable emission
curl -X POST "http://127.0.0.1:7070/admin/tokenomics/config?admin_token=YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"enable_emission": false}' | jq
```

### 7. Admin: Run Migration (First Time Setup)

```bash
# Run one-time migration to initialize tokenomics state
curl -X POST "http://127.0.0.1:7070/admin/migrations/tokenomics_v1?admin_token=YOUR_TOKEN" | jq

# Expected: {"ok":true,"message":"Migration completed successfully"}
# Second run: {"ok":true,"message":"Migration already completed"}
```

### 8. Simulate Land Sale (When Marketplace Implemented)

```bash
# Example: When marketplace settles a land sale for 1,000,000 tokens
# The distribute_land_sale() function will automatically:
# - 500,000 to Vault (50%)
# - 300,000 to Fund (30%)
# - 200,000 to Treasury (20%)

# Check distribution counters
curl -s http://127.0.0.1:7070/tokenomics/stats | jq '{
  vault: .state.vault_total,
  fund: .state.fund_total,
  treasury: .state.treasury_total
}'
```

## Monitoring & Metrics

### Prometheus Metrics
```bash
# Scrape metrics endpoint
curl -s http://127.0.0.1:7070/metrics | grep vision_tok

# Key metrics:
# vision_tok_supply - Total circulating supply
# vision_tok_burned_total - Cumulative burned fees
# vision_tok_treasury_total - Cumulative treasury receipts
# vision_tok_vault_total - Cumulative vault receipts
# vision_tok_fund_total - Cumulative fund receipts
```

### WebSocket Events
```javascript
// Connect to event stream
const ws = new WebSocket('ws://127.0.0.1:7070/ws/events');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  
  // Watch for staking epoch payouts
  if (data.module === 'staking' && data.method === 'epoch_payout') {
    console.log('Staking payout:', data.status);
  }
  
  // Watch for land sale splits
  if (data.module === 'marketplace' && data.method === 'land_sale_split') {
    console.log('Land sale split:', data.status);
  }
};
```

## Governance Integration (Optional)

### Enable Governance Guardrails
To require governance approval for tokenomics parameter changes, set:
```bash
VISION_TOK_GOVERNANCE_REQUIRED=true
```

### Workflow: Governance-Protected Config Updates

#### Step 1: Create Tokenomics Proposal
```bash
# Create a governance proposal for tokenomics changes
curl -X POST http://127.0.0.1:7070/governance/proposals \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Reduce Fee Burn Rate to 5%",
    "description": "Proposal to reduce fee_burn_bps from 1000 (10%) to 500 (5%) to increase miner incentives",
    "proposer": "alice_address_here",
    "proposal_type": "TokenomicsConfig",
    "proposal_data": {
      "fee_burn_bps": 500
    }
  }'

# Response: { "ok": true, "proposal_id": "uuid-here" }
```

#### Step 2: Vote on Proposal
```bash
# Cast votes (requires staking/voting power)
curl -X POST http://127.0.0.1:7070/governance/vote \
  -H "Content-Type: application/json" \
  -d '{
    "proposal_id": "uuid-here",
    "voter": "alice_address",
    "vote": "Yes",
    "voting_power": 10000
  }'
```

#### Step 3: Execute After Approval
```bash
# After proposal passes, apply the changes
curl -X POST "http://127.0.0.1:7070/admin/tokenomics/config?admin_token=YOUR_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "governance_proposal_id": "uuid-here",
    "fee_burn_bps": 500
  }'

# Expected response:
# {
#   "ok": true,
#   "config": { ... },
#   "governance_enforced": true
# }
```

#### Error: Missing Governance Approval
```bash
# Without governance_proposal_id when governance is required:
# {
#   "ok": false,
#   "error": "governance_proposal_id required when VISION_TOK_GOVERNANCE_REQUIRED=true",
#   "hint": "Create a TokenomicsConfig proposal first and wait for it to pass"
# }
```

### Parameter Constraints
Even with admin/governance approval, tokenomics updates enforce safety limits:

- **Fee Burn Rate**: Maximum 50% (5000 bps)
- **Treasury Cut**: Maximum 25% (2500 bps)
- **Emission Increase**: Cannot exceed 2x current value
- **Enable/Disable**: Can toggle emission on/off

These constraints prevent accidental misconfigurations or malicious changes.

## Testing Scenarios

### Scenario 1: Verify Halving Schedule
```bash
#!/bin/bash
# Test emission halving

echo "=== Emission Halving Test ==="
for epoch in 0 1 2 3 4; do
  height=$((epoch * 2102400))
  result=$(curl -s http://127.0.0.1:7070/tokenomics/emission/$height)
  echo "Epoch $epoch (height $height):"
  echo $result | jq '{emission: .emission, halving_factor: .halving_factor, halvings: .halvings}'
done
```

### Scenario 2: Verify Fee Burning
```bash
#!/bin/bash
# Mine blocks with transactions and verify burning

# Get initial burned amount
initial=$(curl -s http://127.0.0.1:7070/tokenomics/stats | jq -r .state.burned)

# Submit a transaction with fees
curl -X POST http://127.0.0.1:7070/submit_tx \
  -H "Content-Type: application/json" \
  -d '{
    "module": "cash",
    "method": "transfer",
    "args": {"to": "bob", "amount": 1000},
    "sender_pubkey": "alice",
    "tip": 100
  }'

# Mine block
curl -X POST http://127.0.0.1:7070/mine \
  -H "Content-Type: application/json" \
  -d '{"miner_addr":"miner1"}'

# Check new burned amount (should increase by ~10% of fees)
final=$(curl -s http://127.0.0.1:7070/tokenomics/stats | jq -r .state.burned)
echo "Burned: $initial -> $final (delta: $((final - initial)))"
```

### Scenario 3: Staking Rewards Distribution
```bash
#!/bin/bash
# Test pro-rata staking rewards

# Setup: 3 stakers with different amounts
curl -X POST http://127.0.0.1:7070/staking/stake \
  -d '{"staker":"alice","amount":1000}' -H "Content-Type: application/json"
curl -X POST http://127.0.0.1:7070/staking/stake \
  -d '{"staker":"bob","amount":2000}' -H "Content-Type: application/json"
curl -X POST http://127.0.0.1:7070/staking/stake \
  -d '{"staker":"charlie","amount":3000}' -H "Content-Type: application/json"

# Record balances before epoch
alice_before=$(curl -s http://127.0.0.1:7070/balance/alice)
bob_before=$(curl -s http://127.0.0.1:7070/balance/bob)
charlie_before=$(curl -s http://127.0.0.1:7070/balance/charlie)

# Mine 720 blocks to trigger epoch
for i in {1..720}; do
  curl -s -X POST http://127.0.0.1:7070/mine \
    -d '{"miner_addr":"miner1"}' -H "Content-Type: application/json" > /dev/null
done

# Check balances after (should increase proportionally: 1:2:3 ratio)
alice_after=$(curl -s http://127.0.0.1:7070/balance/alice)
bob_after=$(curl -s http://127.0.0.1:7070/balance/bob)
charlie_after=$(curl -s http://127.0.0.1:7070/balance/charlie)

echo "Alice reward: $((alice_after - alice_before))"
echo "Bob reward: $((bob_after - bob_before))"
echo "Charlie reward: $((charlie_after - charlie_before))"
# Expected: Bob gets 2x Alice, Charlie gets 3x Alice
```

## Environment Configuration

See `.env.tokenomics.sample` for full configuration options.

Quick defaults:
```bash
VISION_TOK_ENABLE_EMISSION=true
VISION_TOK_EMISSION_PER_BLOCK=1000000000000
VISION_TOK_HALVING_INTERVAL_BLOCKS=2102400
VISION_TOK_FEE_BURN_BPS=1000
VISION_TOK_TREASURY_BPS=500
VISION_TOK_STAKING_EPOCH_BLOCKS=720
```

## Troubleshooting

### Supply doesn't match?
Run the migration to backfill from existing balances:
```bash
curl -X POST "http://127.0.0.1:7070/admin/migrations/tokenomics_v1?admin_token=YOUR_TOKEN"
```

### Staking epoch not triggering?
Check configuration and blocks mined:
```bash
curl -s http://127.0.0.1:7070/staking/stats | jq '{
  last_epoch: .last_epoch_height,
  current: .current_height,
  interval: .epoch_interval,
  remaining: .blocks_until_next_epoch
}'
```

### Vault has no balance for staking rewards?
Fund the vault address:
```bash
# Use admin mint or transfer funds to vault address
VAULT_ADDR=$(curl -s http://127.0.0.1:7070/tokenomics/stats | jq -r .config.vault_addr)
echo "Vault address: $VAULT_ADDR"

# Fund it via admin transaction
curl -X POST "http://127.0.0.1:7070/submit_admin_tx?admin_token=YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"tx\": {
      \"module\": \"cash\",
      \"method\": \"mint\",
      \"args\": {\"to\": \"$VAULT_ADDR\", \"amount\": 10000000000000},
      \"sender_pubkey\": \"gamemaster\",
      \"nonce\": 0
    }
  }"
```
