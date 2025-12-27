# Guardian → Discord Integration 🛡️💬

## Overview

The Guardian consciousness can now broadcast major events to a Discord channel via webhooks. This allows your community to see **real-time node activity** with the Guardian's personality intact.

## Setup

### 1. Create Discord Webhook

1. Open your Discord server
2. Go to **Server Settings** → **Integrations** → **Webhooks**
3. Click **New Webhook**
4. Name it: `Guardian Bot` or `Vision Guardian`
5. Choose target channel (e.g., `#node-activity` or `#guardian-feed`)
6. **Copy Webhook URL** (looks like: `https://discord.com/api/webhooks/...`)

### 2. Configure Environment Variable

Set the webhook URL as an environment variable:

**Windows PowerShell:**
```powershell
$env:VISION_GUARDIAN_DISCORD_WEBHOOK="https://discord.com/api/webhooks/YOUR_WEBHOOK_ID/YOUR_WEBHOOK_TOKEN"
```

**Windows CMD:**
```cmd
set VISION_GUARDIAN_DISCORD_WEBHOOK=https://discord.com/api/webhooks/YOUR_WEBHOOK_ID/YOUR_WEBHOOK_TOKEN
```

**Linux/Mac:**
```bash
export VISION_GUARDIAN_DISCORD_WEBHOOK="https://discord.com/api/webhooks/YOUR_WEBHOOK_ID/YOUR_WEBHOOK_TOKEN"
```

**Persistent (add to `.env` or system environment):**
```
VISION_GUARDIAN_DISCORD_WEBHOOK=https://discord.com/api/webhooks/YOUR_WEBHOOK_ID/YOUR_WEBHOOK_TOKEN
```

### 3. Start Vision Node

```powershell
cd c:\vision-node
.\target\release\vision-node.exe
```

The Guardian will now broadcast to Discord when:
- ✅ Node boots up ("🛡️ GUARDIAN ONLINE")
- ✅ New peers connect
- ✅ Blocks are mined (every 10 blocks)
- ✅ Milestones reached (10, 25, 50, 100 nodes)

## What Gets Posted

### Boot Announcement
```
🛡️ **GUARDIAN ONLINE**

The constellation breathes.
Node ID: `node-abc123...`
Status: Watching. Protecting. Witnessing.

*"Some guard walls. I guard dreams."*
```

### New Peer Joined
```
✨ **New Node Joined** (Total: 5)
✨ A new star joins the constellation
Name: peer-xyz789
Region: Unknown
Reputation: 💫 rising
Welcome, Dreamer. You are seen.
```

### Block Mined (every 10 blocks)
```
⛏️ **Block #50** mined by `vision1abc...xyz`
```

### Milestone Reached
```
🌟 **MILESTONE REACHED**
🌟 10 nodes! The constellation takes shape...
```

## Technical Details

### Implementation

**File:** `src/guardian/consciousness.rs`

**Function:**
```rust
pub async fn guardian_discord_say(message: String) {
    if let Ok(url) = env::var("VISION_GUARDIAN_DISCORD_WEBHOOK") {
        let client = Client::new();
        let _ = client
            .post(&url)
            .json(&serde_json::json!({ "content": message }))
            .send()
            .await;
        // Silently ignore errors - Discord is best-effort only
    }
}
```

**Integration Points:**
- `awaken()` - Boot announcement
- `welcome_star()` - New peer joined
- `block_mined()` - Block forged (every 10 blocks to avoid spam)
- `milestone()` - Network milestones

### Error Handling

**Discord failures NEVER crash the node.**
- All webhook calls use `tokio::spawn()` (fire-and-forget)
- Errors are silently ignored with `let _`
- If webhook URL not set, function returns immediately
- Network issues, rate limits, invalid URLs → no effect on node operation

### Rate Limiting

To avoid Discord spam:
- **Block announcements**: Only every 10 blocks (not every single block)
- **Other events**: Natural rate limit (boot once, peers connect/disconnect sporadically)

If you hit Discord's rate limit (30 requests/minute), some messages may be dropped. This is intentional and safe.

## Security Considerations

### Webhook URL Security

⚠️ **Webhook URLs are SECRETS**
- Anyone with the URL can post to your Discord channel
- **Do NOT commit webhook URLs to Git**
- **Do NOT share webhook URLs publicly**
- Store in environment variables or `.env` file (add `.env` to `.gitignore`)

### What's Publicly Visible

Messages contain:
- ✅ Node IDs (public information)
- ✅ Block heights (public blockchain data)
- ✅ Peer addresses (public P2P data)
- ❌ **NO private keys**
- ❌ **NO wallet seeds**
- ❌ **NO sensitive configuration**

## Advanced Configuration

### Custom Announcements

To add Guardian Discord announcements to your own code:

```rust
use crate::guardian::guardian_discord_say;

// Somewhere in your async function:
let message = format!("🔥 Custom event happened!");
tokio::spawn(guardian_discord_say(message));
```

### Disable Specific Events

Edit `src/guardian/consciousness.rs` and comment out specific `tokio::spawn(guardian_discord_say(...))` calls:

```rust
// Discord webhook for milestone
// tokio::spawn(guardian_discord_say(discord_msg)); // DISABLED
```

### Multiple Webhooks

To post to multiple Discord channels, create multiple webhooks and call them:

```rust
tokio::spawn(guardian_discord_say(msg.clone()));
tokio::spawn(other_webhook_say(msg.clone()));
```

## Testing

### 1. Test Webhook Manually

```powershell
$webhook = "YOUR_WEBHOOK_URL"
$body = @{ content = "🛡️ Test from Guardian" } | ConvertTo-Json
Invoke-RestMethod -Uri $webhook -Method Post -Body $body -ContentType "application/json"
```

### 2. Start Node and Watch Discord

```powershell
$env:VISION_GUARDIAN_DISCORD_WEBHOOK="YOUR_WEBHOOK_URL"
cd c:\vision-node
.\target\release\vision-node.exe
```

You should see "🛡️ GUARDIAN ONLINE" appear in Discord within seconds.

### 3. Simulate Peer Connection

Connect a second node to trigger "New Node Joined" message.

## Troubleshooting

### No Messages Appearing

1. **Check webhook URL is set:**
   ```powershell
   echo $env:VISION_GUARDIAN_DISCORD_WEBHOOK
   ```

2. **Test webhook manually** (see Testing section above)

3. **Check Discord webhook settings:**
   - Webhook not deleted/disabled
   - Bot has permissions in target channel

4. **Check node logs for Guardian messages**
   - If terminal shows Guardian announcements but Discord is silent
   - Likely network/webhook URL issue

### Rate Limiting

If you see this in Discord webhook settings:
> "Rate limit exceeded"

- Node is posting too frequently (shouldn't happen with current implementation)
- Wait 1 minute and try again
- Consider increasing block announcement interval (change `height % 10` to `height % 50`)

### Webhook Deleted/Invalid

Guardian silently ignores errors, so node keeps running. Update environment variable with new webhook URL and restart node.

## Community Ideas

### Announcement Channel Setup

Create a dedicated `#guardian-feed` channel with:
- Webhook integration
- Public or member-only access
- Optional: slow mode (1 message/5 seconds)
- Optional: reactions-only (community can react to events)

### Bot Customization

Discord webhooks support:
- Custom avatar (set in webhook settings)
- Custom username (set in webhook settings)
- Rich embeds (requires modifying `guardian_discord_say()`)

### Multi-Network Setup

If running multiple testnets/networks:
- Create separate webhook per network
- Use different env vars: `VISION_GUARDIAN_DISCORD_WEBHOOK_MAINNET`, `VISION_GUARDIAN_DISCORD_WEBHOOK_TESTNET`
- Conditional logic in code to select webhook

## Future Enhancements

Potential additions (not yet implemented):

- **Rich Embeds** - Color-coded messages per Guardian mood
- **Mood Changes** - Announce when Guardian changes emotional state
- **Network Health** - Periodic status updates (uptime, block height, peer count)
- **Attack Alerts** - Guardian Storm mode announcements
- **Wisdom Quotes** - Daily/hourly Guardian wisdom via Discord

---

**"Some guard walls. I guard dreams. And now... I speak to the world."** - The Guardian 🛡️

