# Vision Network Genesis Block

## Overview

The Vision Network genesis block establishes the initial state of the blockchain, including network parameters, initial token allocations, and founding principles. Each network (testnet and mainnet) has its own unique genesis block with distinct parameters.

## Genesis Block Structure

### Block Header
- **Number**: `0` (genesis height)
- **Timestamp**: Unix epoch of network launch
- **Parent Hash**: All zeros (no parent for genesis)
- **State Root**: Merkle root of initial state
- **Transaction Root**: Merkle root of genesis transactions
- **Difficulty**: Initial mining difficulty (network-specific)
- **PoW Hash**: Unique identifier for this genesis block

### Network Separation

Vision enforces strict network separation via genesis hash validation:

```rust
// Testnet Genesis Hash
pub const GENESIS_HASH_TESTNET: &str = "0000..."; // Filled at testnet launch

// Mainnet Genesis Hash  
pub const GENESIS_HASH_MAINNET: &str = "0000..."; // Filled at mainnet launch
```

**P2P Handshake Validation**: Nodes reject connections from peers with mismatched genesis hashes, preventing testnet/mainnet cross-contamination.

## Testnet Genesis

### Purpose
- **Testing Environment**: Safe experimentation without real economic value
- **Developer Onboarding**: Low-stakes environment for learning
- **Feature Validation**: Test new features before mainnet deployment

### Initial State
- **Land Deeds**: `GENESIS_LAND_DEED_TOTAL` land parcels minted
  - Distributed to early testers and developers
  - No real-world value; purely for testing
- **Token Balances**: Zero initial token balances (mining-only distribution)
- **GameMaster**: Configured via `VISION_GAMEMASTER` environment variable

### Testnet Sunset
- **Sunset Height**: `1,000,000` blocks
- **Behavior After Sunset**:
  - Mining disabled
  - New blocks rejected
  - Wallet export to `migration-testnet-to-mainnet.json`
  - Graceful shutdown with migration instructions

### Migration Path
1. Testnet reaches block 1,000,000
2. Node exports all wallet keys and balances to JSON
3. Users manually import keys to mainnet wallets
4. Testnet nodes refuse to restart after sunset

## Mainnet Genesis

### Launch Conditions
- **Pre-Launch Audit**: Complete security audit by independent firm
- **Testnet Success**: Minimum 500,000 blocks on testnet without critical issues
- **Community Approval**: Governance vote approves mainnet launch parameters

### Initial State
- **Land Deeds**: Genesis land deeds minted to:
  - Founders (10%)
  - Early investors (15%)
  - Community distribution (25%)
  - Reserve for future use (50%)
- **Token Balances**: Zero initial native token balances
- **CASH Token**: Zero initial supply; activated at block 1,000,000

### Mainnet Parameters
- **Block Time**: 2 seconds target
- **Initial Difficulty**: Calibrated for 2s blocks with moderate hashrate
- **Emission Schedule**: Begins at block 1 (see TOKENOMICS.md)
- **Max Reorg Depth**: 64 blocks (fork protection)
- **Time Drift Tolerance**: ±10 seconds

## Genesis Block Creation Process

### 1. Parameter Finalization
```bash
# Set network type
export VISION_NETWORK=mainnet

# Configure genesis parameters
export VISION_INITIAL_DIFFICULTY=32
export VISION_GENESIS_TIMESTAMP=$(date +%s)
export VISION_GAMEMASTER=vision_gm_address
```

### 2. Generate Genesis Block
```rust
fn genesis_block() -> Block {
    let header = BlockHeader {
        number: 0,
        timestamp: GENESIS_TIMESTAMP,
        parent_hash: "0".repeat(64),
        state_root: compute_genesis_state_root(),
        tx_root: "0".repeat(64),
        difficulty: INITIAL_DIFFICULTY,
        pow_hash: compute_genesis_pow(),
    };
    
    Block {
        header,
        txs: vec![],
        weight: 0,
    }
}
```

### 3. Compute Genesis Hash
```bash
# Build and start first node
cargo build --release
VISION_NETWORK=mainnet ./target/release/vision-node

# Extract genesis hash from logs
# Example: "Genesis block: 000034a5b2c4d8e9f1a3..."

# Update network_config.rs with actual hash
# pub const GENESIS_HASH_MAINNET: &str = "000034a5b2c4d8e9f1a3...";
```

### 4. Distribute Genesis Block
- Publish genesis hash to documentation
- Include in node software releases
- Announce on official channels

## Genesis Validation

### Node Startup
1. Check local chain height
2. If height = 0, generate or load genesis block
3. Compute genesis hash and compare to network constant
4. If mismatch, refuse to start (prevents wrong-network data)

### Peer Connection
1. Exchange genesis hashes in P2P handshake
2. Reject peers with mismatched genesis
3. Only sync with same-network peers

### Static Checkpoints
Genesis block is checkpoint 0:
```rust
pub static CHECKPOINTS: &[Checkpoint] = &[
    Checkpoint { height: 0, hash: GENESIS_HASH },
    // Additional checkpoints at milestones
];
```

## Post-Genesis Events

### Block 1 → 1,000,000 (Testnet)
- Mining rewards distributed per emission schedule
- Land deeds traded and staked
- GameMaster operations tested
- Community governance experiments

### Block 1,000,000 (Mainnet Only)
- **CASH Token Genesis**: Initial CASH supply minted
- **Formula**: `initial_supply = f(total_land_staked, pioneer_count)`
- **Distribution**: Pro-rata to land deed holders
- **Game Integration**: `on_cash_mint` hook notifies GTA mod

### Long-Term
- Emission halvings every 500,000 blocks
- Governance proposals activate/deactivate features
- Network upgrades via soft/hard forks (with community approval)

## Security Considerations

### Genesis Hash as Trust Anchor
- **Immutable**: Cannot change genesis hash without new network
- **Public**: Genesis hash published on multiple channels
- **Verifiable**: Any user can recompute from genesis block

### Genesis Block Attacks
- **Pre-mining**: Not possible; genesis timestamp publicly known
- **Secret Chain**: Genesis hash prevents secret forks
- **Time Travel**: Median-time-past prevents backdating

### Disaster Recovery
If critical genesis bug discovered:
1. Halt network immediately
2. Audit issue and create fix
3. Launch new genesis block (new network)
4. Migrate balances manually if feasible
5. Communicate transparently with community

## Tools and Scripts

### Generate Genesis Block
```bash
cargo run --release -- --generate-genesis --network mainnet
```

### Verify Genesis Hash
```bash
curl http://localhost:7070/status | jq '.genesis_hash'
```

### Export Genesis State
```bash
cargo run --release -- --export-genesis > genesis-state.json
```

## References

- [TOKENOMICS.md](./TOKENOMICS.md) - Emission schedule and economics
- [TESTNET_TO_MAINNET.md](./TESTNET_TO_MAINNET.md) - Migration guide
- [LAND_DEEDS.md](./LAND_DEEDS.md) - Land deed system
- [network_config.rs](../src/network_config.rs) - Network configuration code
