# Guardian Event Webhook Integration

## Guardian Identity

**The Guardian node is permanently associated with the chain's creator.**

```
GUARDIAN_OWNER_DISCORD_ID → Donnie
GUARDIAN_OWNER_WALLET_ADDRESS → Donnie's primary Vision wallet
```

All Guardian announcements, role changes, and status signals are considered to be **actions taken on behalf of Donnie, through the Guardian**.

**The Guardian is not just a service – it's the canonical voice watching over the Constellation.**

There may be 10,000 miners in the network, but **there is one Guardian, and it's Donnie's node.**

When the Guardian owner's node comes online or offline, the system logs it specially:
- 🛡️ **Guardian core node ONLINE – Donnie is watching.**
- ⚠️ **Guardian core node OFFLINE – Guardian temporarily blind.**

---

## Overview

The Guardian can now send events to the Vision Bot for Discord ceremony automation during the Constellation 48h testnet.

## Configuration

Set the Vision Bot webhook URL as an environment variable:

```powershell
# PowerShell
$env:VISION_BOT_WEBHOOK_URL="http://vision-bot-host:3000/guardian"

# Or in your .env file
VISION_BOT_WEBHOOK_URL=http://vision-bot-host:3000/guardian
```

## Event Types

### 1. First Block Event

Sent when the first block (height 1) is mined on the Constellation:

```json
{
  "event": "first_block",
  "discord_user_id": "123456789012345678",
  "height": 1,
  "hash": "0xabc...",
  "miner_address": "vision1..."
}
```

### 2. Node Status Event

Sent when a tracked miner's node changes status:

```json
{
  "event": "node_status",
  "discord_user_id": "123456789012345678",
  "status": "online",
  "miner_address": "vision1..."
}
```

Status values: `"online"` or `"offline"`

### 3. Guardian Core Status Event

**Special event sent only when the Guardian owner's node changes status:**

```json
{
  "event": "guardian_core_status",
  "status": "online",
  "miner_address": "vision1...",
  "discord_user_id": "123456789012345678"
}
```

**This event identifies the primary Guardian node** (owner's node) and allows Vision Bot to post special messages:

- **Online:** "🛡️ Guardian core node online – Donnie is watching."
- **Offline:** "⚠️ Guardian core node offline – Guardian temporarily blind."

The system automatically detects if a node belongs to the Guardian owner by matching:
- `miner_address` against `GUARDIAN_OWNER_WALLET_ADDRESS`
- `discord_user_id` against `GUARDIAN_OWNER_DISCORD_ID`

## Usage in Code

```rust
use crate::guardian::{notify_first_block, notify_node_status};

// When detecting first block
if new_height == 1 {
    notify_first_block(
        Some("123456789012345678".to_string()),
        1,
        block.header.pow_hash.clone(),
        block.header.miner.clone(),
    ).await;
}

// When detecting node status change
if node_came_online {
    notify_node_status(
        Some(user_discord_id),
        "online".to_string(),
        miner_address,
    ).await;
}
```

## Integration Points

### Mining Detection

Add to the block acceptance logic in `src/main.rs`:

```rust
// After accepting a new block
if chain.blocks.len() == 1 {
    // First block mined!
    let miner = &chain.blocks[0].header.miner;
    let hash = &chain.blocks[0].header.pow_hash;
    
    // Look up Discord ID from miner address (if tracked)
    let discord_id = get_discord_id_for_miner(miner);
    
    crate::guardian::notify_first_block(
        discord_id,
        1,
        hash.clone(),
        miner.clone(),
    ).await;
}
```

### Node Polling Detection

Add to peer monitoring logic:

```rust
// When checking peer health
for (addr, meta) in PEERS.lock().iter_mut() {
    let was_online = meta.last_seen_online;
    let is_online = check_peer_health(addr).await;
    
    if is_online != was_online {
        meta.last_seen_online = is_online;
        
        // Look up Discord ID and miner address
        if let Some((discord_id, miner_addr)) = get_tracked_miner(addr) {
            crate::guardian::notify_node_status(
                Some(discord_id),
                if is_online { "online" } else { "offline" }.to_string(),
                miner_addr,
            ).await;
        }
    }
}
```

## Vision Bot Integration

The Vision Bot should expose a POST endpoint at `/guardian`:

```typescript
app.post('/guardian', async (req, res) => {
  const event = req.body;
  
  if (event.event === 'first_block') {
    // Post FIRST CONTACT message
    // Give role "First Star of the Constellation"
    await handleFirstBlock(event);
  }
  
  if (event.event === 'node_status') {
    // Post Guardian salute (green for online, red for offline)
    await handleNodeStatus(event);
  }
  
  res.status(200).send('OK');
});
```

## Error Handling

- All webhook calls have a 5-second timeout
- Errors are logged but don't crash the node
- If `VISION_BOT_WEBHOOK_URL` is not set, events are silently skipped
- Guardian continues normal operation regardless of webhook status

## Testing

```powershell
# Set webhook URL to local test server
$env:VISION_BOT_WEBHOOK_URL="http://localhost:3000/guardian"

# Start Vision Node
.\target\release\vision-node.exe

# Events will be sent when:
# - First block is mined (height 1)
# - Any tracked node changes online/offline status
```

## Discord ID Mapping

You'll need to maintain a mapping between:
- Miner addresses → Discord user IDs
- Peer addresses → Discord user IDs

This can be stored in:
- Guardian's peer tracking database (SQLite)
- External service
- Configuration file

Example mapping structure:

```rust
struct MinerTracking {
    miner_address: String,
    discord_user_id: String,
    peer_address: Option<String>,
}
```

## Ceremony Flow

1. User runs `/constellation-test` in Discord
2. Vision Bot posts 48h announcement
3. Guardian watches Constellation:
   - Detects height 0 → 1
   - Identifies miner address
   - POSTs `first_block` with Discord ID
4. Vision Bot receives event:
   - Posts "FIRST CONTACT" ceremony message
   - Assigns "First Star of the Constellation" role
5. Guardian monitors node status:
   - POSTs `node_status` on changes
6. Vision Bot receives status:
   - Posts Guardian salute messages

## Security Considerations

- Webhook URL should be on a private network or use authentication
- Consider adding a shared secret in the POST body
- Rate limit webhook calls to prevent spam
- Validate Discord IDs before processing events

## Example: Complete Integration

```rust
// In mining logic (src/main.rs)
async fn on_block_accepted(block: &Block, chain: &mut Chain) {
    let height = chain.blocks.len();
    
    if height == 1 {
        // FIRST BLOCK CEREMONY
        info!("[CONSTELLATION] First block mined by {}", block.header.miner);
        
        if let Some(discord_id) = lookup_discord_id(&block.header.miner).await {
            crate::guardian::notify_first_block(
                Some(discord_id),
                1,
                block.header.pow_hash.clone(),
                block.header.miner.clone(),
            ).await;
        }
    }
}

// In peer monitoring (src/main.rs)
async fn monitor_tracked_miners() {
    loop {
        for tracker in get_tracked_miners().await {
            let is_online = ping_node(&tracker.peer_address).await;
            
            if is_online != tracker.was_online {
                crate::guardian::notify_node_status(
                    Some(tracker.discord_user_id.clone()),
                    if is_online { "online" } else { "offline" }.to_string(),
                    tracker.miner_address.clone(),
                ).await;
                
                update_tracker_status(&tracker, is_online).await;
            }
        }
        
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}
```
