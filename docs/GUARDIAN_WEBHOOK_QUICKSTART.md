# Guardian Event Webhooks - Quick Reference

## Setup (5 minutes)

### 1. Configure Webhook URL

```powershell
# On Guardian Node
$env:VISION_BOT_WEBHOOK_URL="http://vision-bot-host:3000/guardian"
```

### 2. Events Sent

**First Block Event** (when height reaches 1):
```json
{
  "event": "first_block",
  "discord_user_id": "123456789012345678",
  "height": 1,
  "hash": "0xabc...",
  "miner_address": "vision1..."
}
```

**Node Status Event** (when tracked node changes):
```json
{
  "event": "node_status",
  "discord_user_id": "123456789012345678",
  "status": "online",
  "miner_address": "vision1..."
}
```

### 3. Vision Bot Endpoint

```typescript
app.post('/guardian', async (req, res) => {
  const { event, discord_user_id } = req.body;
  
  if (event === 'first_block') {
    // Post FIRST CONTACT ceremony
    // Give "First Star of the Constellation" role
  }
  
  if (event === 'node_status') {
    // Post Guardian salute (green online / red offline)
  }
  
  res.send('OK');
});
```

## Integration Flow

1. User runs `/constellation-test` in Discord
2. Bot stores: `{ discord_user_id, miner_address }`
3. Guardian watches Constellation:
   - **First block mined** → POST `first_block`
   - **Node status changes** → POST `node_status`
4. Bot receives events → Posts ceremony messages

## Files Added

- `src/guardian/events.rs` - Event types and sending logic
- `src/guardian/integration_example.rs` - Complete integration examples
- `docs/GUARDIAN_EVENT_WEBHOOKS.md` - Full documentation

## Next Steps

1. **Add tracking database** - Store Discord ID ↔ Miner Address mapping
2. **Integrate into mining logic** - Detect first block and send event
3. **Add node monitoring** - Background task to check peer health
4. **Test with Vision Bot** - Verify ceremony messages work

## Testing

```powershell
# Test event sending (with mock webhook server)
cargo test guardian_events

# Start node with webhook configured
$env:VISION_BOT_WEBHOOK_URL="http://localhost:3000/guardian"
.\target\release\vision-node.exe
```

## Security

- Use private network for webhook URL
- Add authentication token if needed
- Rate limit webhook calls
- Validate Discord IDs

## Ceremony Lore

**First Block** → "FIRST CONTACT" + role assignment
**Node Online** → Green salute from Guardian
**Node Offline** → Red alert from Guardian

The Guardian watches. The Constellation awakens. ✨
