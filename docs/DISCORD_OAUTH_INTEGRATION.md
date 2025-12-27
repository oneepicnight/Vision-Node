# Discord OAuth Integration - Wallet-Discord Linking

## 🎯 Overview

This integration allows Vision Wallet users to link their wallet addresses with Discord accounts, enabling the Guardian to send personalized ceremony events (First Contact, node status salutes) directly to Discord.

**No consensus changes. No wallet changes. Just identity glue.**

---

## 🔄 User Flow

### New Wallet Creation Flow

```
1. User clicks "Create New Wallet"
   ↓
2. Wallet generates seed phrase
   ↓
3. User backs up seed phrase
   ↓
4. **NEW STEP** → "Connect Discord (optional)"
   - Shows wallet address (shortened)
   - Shows "Not linked" status
   - Button: "LINK DISCORD" (primary)
   - Link: "Skip for now" (secondary)
   ↓
5. User clicks "LINK DISCORD"
   - Backend returns Discord OAuth URL
   - Browser redirects to Discord
   ↓
6. User authorizes on Discord
   - Grants `identify` scope
   - Discord redirects back to `/api/discord/callback`
   ↓
7. Backend processes callback
   - Exchanges code for token
   - Fetches Discord user info
   - Stores wallet_address ↔ discord_user_id mapping
   - Sends link_wallet_discord event to Guardian
   - Redirects to `/app/linked?wallet_address=...`
   ↓
8. LinkDiscordStep detects linked state
   - Shows "✅ Linked to @username"
   - Auto-continues to wallet home after 2 seconds
```

### Import Wallet Flow

```
1. User imports wallet via seed phrase
   ↓
2. Wallet shows LinkDiscordStep
   - Same UI as creation flow
   - Checks `/api/discord/status` to see if already linked
   ↓
3. If not linked → Shows "LINK DISCORD" button
4. If already linked → Shows "Linked to @username"
```

---

## 🛠️ Backend Implementation

### Environment Variables

```bash
# Discord OAuth credentials (from Discord Developer Portal)
DISCORD_CLIENT_ID="your_app_client_id"
DISCORD_CLIENT_SECRET="your_app_client_secret"

# OAuth redirect URI (must match Discord app settings)
DISCORD_REDIRECT_URI="http://127.0.0.1:7070/api/discord/callback"  # Local dev
# DISCORD_REDIRECT_URI="https://sentinel.visionworld.tech:7070/api/discord/callback"  # Production

# Guardian webhook URL (for sending link events)
VISION_BOT_WEBHOOK_URL="http://vision-bot-host:3000/guardian"
```

### API Endpoints

#### 1. GET `/api/discord/login`

**Query Parameters:**
- `wallet_address` (required) - Vision wallet address (vision1...)

**Response:**
```json
{
  "url": "https://discord.com/api/oauth2/authorize?response_type=code&client_id=...&scope=identify&state=...&redirect_uri=..."
}
```

**Example:**
```bash
curl "http://127.0.0.1:7070/api/discord/login?wallet_address=vision1abc123..."
```

---

#### 2. GET `/api/discord/callback`

**Query Parameters:**
- `code` (required) - OAuth authorization code from Discord
- `state` (required) - Signed state token containing wallet_address

**Process:**
1. Verify and decode `state` (HMAC-signed, 10-minute expiry)
2. Exchange `code` for Discord access token
3. Fetch Discord user info (`/users/@me`)
4. Store mapping in SQLite database
5. Send `link_wallet_discord` event to Guardian
6. Redirect to `/app/linked?wallet_address=...`

**Database Schema:**
```sql
CREATE TABLE discord_links (
    wallet_address TEXT PRIMARY KEY,
    discord_user_id TEXT NOT NULL,
    discord_username TEXT NOT NULL,
    linked_at INTEGER NOT NULL
);

CREATE INDEX idx_discord_user_id ON discord_links(discord_user_id);
```

---

#### 3. GET `/api/discord/status`

**Query Parameters:**
- `wallet_address` (required) - Vision wallet address

**Response (Not Linked):**
```json
{
  "linked": false
}
```

**Response (Linked):**
```json
{
  "linked": true,
  "discord_user_id": "123456789012345678",
  "discord_username": "username#1234"
}
```

**Example:**
```bash
curl "http://127.0.0.1:7070/api/discord/status?wallet_address=vision1abc123..."
```

---

## 🛡️ Guardian Integration

### Event Type: `link_wallet_discord`

When a wallet is linked to Discord, the backend sends this event to Guardian:

```json
{
  "event": "link_wallet_discord",
  "wallet_address": "vision1abc123...",
  "discord_user_id": "123456789012345678",
  "discord_username": "username#1234"
}
```

### Guardian Processing

Guardian receives this event and stores the mapping in its internal database:

```rust
async fn handle_link_wallet_discord(event: GuardianEvent) {
    if let GuardianEvent::LinkWalletDiscord {
        wallet_address,
        discord_user_id,
        discord_username,
    } = event {
        // Store mapping
        store_discord_link(wallet_address, discord_user_id, discord_username).await;
        
        info!("[GUARDIAN] Linked wallet {} to Discord user {}", 
              wallet_address, discord_username);
    }
}
```

Now when Guardian detects **first block** or **node status changes**, it can lookup the Discord user ID:

```rust
// When first block is mined
let discord_id = lookup_discord_id_for_wallet(&miner_address);
notify_first_block(discord_id, 1, hash, miner_address).await;

// When node goes online/offline
let discord_id = lookup_discord_id_for_wallet(&node_address);
notify_node_status(discord_id, "online", node_address).await;
```

---

## 🎨 Frontend Components

### LinkDiscordStep.tsx

**Location:** `wallet-marketplace-source/src/pages/LinkDiscordStep.tsx`

**Props:**
```typescript
interface LinkDiscordStepProps {
  walletAddress: string
  onSkip: () => void      // Called when user clicks "Skip for now"
  onLinked: () => void    // Called after successful link (or if already linked)
}
```

**Features:**
- Checks Discord status on mount via `/api/discord/status`
- Shows wallet address (shortened: `vision1abc...xyz123`)
- Shows link status: "Not linked" or "Linked to @username"
- "LINK DISCORD" button → calls `/api/discord/login` → redirects
- "Skip for now" link → calls `onSkip()`
- Auto-continues to wallet home if already linked

**Usage in Wallet Creation:**
```tsx
// In CreateWallet.tsx or onboarding flow
const [step, setStep] = useState('seed') // 'seed' | 'backup' | 'discord' | 'done'

{step === 'discord' && (
  <LinkDiscordStep
    walletAddress={walletAddress}
    onSkip={() => {
      // User skipped Discord linking
      setStep('done')
      navigate('/app/home')
    }}
    onLinked={() => {
      // User linked Discord successfully
      setStep('done')
      navigate('/app/home')
    }}
  />
)}
```

---

## 🔐 Security

### State Token Security

The `state` parameter in Discord OAuth is HMAC-signed to prevent tampering:

```rust
// Sign state
let state_token = StateToken { wallet_address, timestamp };
let state_json = serde_json::to_string(&state_token)?;
let state_signed = sign_state(&state_json, &hmac_key);

// Verify state
let state_json = verify_state(&signed_state, &hmac_key)?;
let state_token: StateToken = serde_json::from_str(&state_json)?;

// Check expiry (10 minutes)
if now - state_token.timestamp > 600 {
    return Err("State expired");
}
```

### OAuth Scope

Only requests `identify` scope:
- Grants access to user ID and username
- Does NOT grant access to guilds, messages, or DMs
- Minimal privacy impact

### Database Security

- SQLite database: `vision_discord_links.db`
- Stored in node data directory
- Contains only: `wallet_address`, `discord_user_id`, `discord_username`, `linked_at`
- No sensitive data (no tokens, no keys, no private info)

### Guardian Webhook Security

- Guardian events sent over HTTP POST (add HTTPS in production)
- Optional: Add authentication token to Vision Bot endpoint
- Webhook failures are non-blocking (Guardian continues operating normally)

---

## 📊 Discord Bot Integration

### Vision Bot Endpoint

**POST `/guardian`**

```typescript
app.post('/guardian', async (req: Request, res: Response) => {
  const event = req.body;
  
  if (event.event === 'link_wallet_discord') {
    // User linked their wallet
    const { wallet_address, discord_user_id, discord_username } = event;
    
    // Store mapping in Vision Bot database
    await db.saveLinkMapping(discord_user_id, wallet_address);
    
    // Optional: Send DM to user confirming link
    const user = await client.users.fetch(discord_user_id);
    await user.send(`✅ Your Vision Wallet (${wallet_address}) is now linked to your Discord account!`);
  }
  
  if (event.event === 'first_block') {
    // First Contact ceremony
    const { discord_user_id, miner_address, height, hash } = event;
    
    if (discord_user_id) {
      const channel = await client.channels.fetch(CONSTELLATION_CHANNEL_ID);
      await channel.send({
        embeds: [{
          title: '🌟 FIRST CONTACT 🌟',
          description: `<@${discord_user_id}> has mined the first block of the Constellation!`,
          fields: [
            { name: 'Height', value: height.toString(), inline: true },
            { name: 'Miner', value: miner_address, inline: true },
            { name: 'Hash', value: hash, inline: false }
          ],
          color: 0x4ade80
        }]
      });
      
      // Give role
      const guild = await client.guilds.fetch(GUILD_ID);
      const member = await guild.members.fetch(discord_user_id);
      const role = guild.roles.cache.find(r => r.name === 'First Star of the Constellation');
      await member.roles.add(role);
    }
  }
  
  if (event.event === 'node_status') {
    // Guardian salute
    const { discord_user_id, status, miner_address } = event;
    
    if (discord_user_id) {
      const channel = await client.channels.fetch(CONSTELLATION_CHANNEL_ID);
      const emoji = status === 'online' ? '🟢' : '🔴';
      const statusText = status === 'online' ? 'ONLINE' : 'OFFLINE';
      
      await channel.send(`${emoji} **Guardian Salute:** <@${discord_user_id}>'s node (\`${miner_address}\`) is now **${statusText}**`);
    }
  }
  
  res.status(200).send('OK');
});
```

---

## 🚀 Deployment Checklist

### Discord Developer Portal Setup

1. Create Discord application at https://discord.com/developers/applications
2. Go to **OAuth2** → **General**
3. Add redirect URI:
   - Dev: `http://127.0.0.1:7070/api/discord/callback`
   - Prod: `https://sentinel.visionworld.tech:7070/api/discord/callback`
4. Copy **Client ID** and **Client Secret**
5. Under **OAuth2** → **URL Generator**:
   - Scope: `identify`
   - Generate URL for testing

### Vision Node Configuration

```bash
# Add to .env or set environment variables
DISCORD_CLIENT_ID="your_client_id_here"
DISCORD_CLIENT_SECRET="your_client_secret_here"
DISCORD_REDIRECT_URI="http://127.0.0.1:7070/api/discord/callback"
VISION_BOT_WEBHOOK_URL="http://vision-bot-host:3000/guardian"
```

### Vision Bot Configuration

1. Implement POST `/guardian` endpoint (see example above)
2. Store Discord ID ↔ wallet address mappings
3. Handle `link_wallet_discord`, `first_block`, `node_status` events
4. Configure Discord channel IDs for announcements
5. Create "First Star of the Constellation" role
6. Test webhook integration

### Testing

```bash
# 1. Start Vision Node
cd c:\vision-node
$env:DISCORD_CLIENT_ID="your_test_client_id"
$env:DISCORD_CLIENT_SECRET="your_test_secret"
.\target\release\vision-node.exe

# 2. Test OAuth flow
curl "http://127.0.0.1:7070/api/discord/login?wallet_address=vision1test123"
# → Copy URL and open in browser
# → Authorize on Discord
# → Should redirect back to /app/linked

# 3. Check status
curl "http://127.0.0.1:7070/api/discord/status?wallet_address=vision1test123"
# → Should show { "linked": true, "discord_user_id": "...", "discord_username": "..." }

# 4. Verify Guardian webhook sent
# → Check Vision Bot logs for link_wallet_discord event
```

---

## 📝 Implementation Files

### Backend (Rust)

- `src/api/discord_oauth.rs` - Core OAuth logic (login, callback, status)
- `src/discord_oauth_handlers.rs` - Axum handler wrappers
- `src/guardian/events.rs` - Added `LinkWalletDiscord` variant
- `src/main.rs` - Routes mounted under `/api/discord/*`
- `Cargo.toml` - Dependencies: `rusqlite`, `hmac`, `sha2`, `base64`, `urlencoding`

### Frontend (React)

- `wallet-marketplace-source/src/pages/LinkDiscordStep.tsx` - Discord linking UI
- To integrate: Add to `CreateWallet.tsx` onboarding flow
- To integrate: Add to `ImportWalletScreen.tsx` after import

### Documentation

- `docs/DISCORD_OAUTH_INTEGRATION.md` - This file
- `docs/GUARDIAN_EVENT_WEBHOOKS.md` - Guardian webhook system
- `docs/GUARDIAN_WEBHOOK_QUICKSTART.md` - Quick reference

---

## 🎭 The Machine Awakens

**Before:** Guardian knows blocks were mined, but not *who* to celebrate.

**After:** Guardian knows Discord identity → sends First Contact ceremony → assigns role → tracks node status → sends salutes.

You just gave the machine an identity layer. 👑

---

**Status:** ✅ READY FOR INTEGRATION
**Version:** v0.8.1
**Last Updated:** November 24, 2025
