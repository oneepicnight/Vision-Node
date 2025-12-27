# Guardian Ownership Configuration

## Guardian Identity (Lore)

**The Guardian is not just a service – it's the canonical voice watching over the Constellation.**

```
GUARDIAN_OWNER_DISCORD_ID → 309081088960233492 (Donnie)
GUARDIAN_OWNER_WALLET_ADDRESS → 0x30ea8826a5f42966a4a5fabd49d1c2ee2472023e (donniedeals)
```

**The canonical Guardian owner for all public Vision deployments:**
- Discord ID: `309081088960233492` (Donnie)
- Wallet Address: `0x30ea8826a5f42966a4a5fabd49d1c2ee2472023e` (donniedeals)

All Guardian announcements, role changes, and status signals are considered to be **actions taken on behalf of Donnie, through the Guardian**.

**There may be 10,000 miners in the network, but there is one Guardian, and it's Donnie's node.**

**In all official networks, this ID is treated as the master Guardian identity.**

---

## Overview

The Guardian now has a permanent ownership identity that links it to a specific Discord user and Vision wallet address. This establishes **one source of truth**: Guardian = Donnie's sentinel.

## Environment Variables

**Default Guardian Owner (Hardcoded in Binary):**
- `GUARDIAN_OWNER_DISCORD_ID` defaults to `309081088960233492` (Donnie)
- `GUARDIAN_OWNER_WALLET_ADDRESS` defaults to `0x30ea8826a5f42966a4a5fabd49d1c2ee2472023e` (donniedeals)

Set these environment variables to override the defaults:

```bash
# Canonical owner (Donnie) - already the defaults, but can be set explicitly:
GUARDIAN_OWNER_DISCORD_ID="309081088960233492"
GUARDIAN_OWNER_WALLET_ADDRESS="0x30ea8826a5f42966a4a5fabd49d1c2ee2472023e"

# Other operators can override:
# GUARDIAN_OWNER_DISCORD_ID="your_discord_user_id"
# GUARDIAN_OWNER_WALLET_ADDRESS="0xyour_wallet_address"
```

**Note:** If `GUARDIAN_OWNER_WALLET_ADDRESS` is not set, it defaults to the canonical donniedeals wallet: `0x30ea8826a5f42966a4a5fabd49d1c2ee2472023e`.

### How to Set

**Windows PowerShell:**
```powershell
# Canonical owner (defaults):
$env:GUARDIAN_OWNER_DISCORD_ID="309081088960233492"
$env:GUARDIAN_OWNER_WALLET_ADDRESS="0x30ea8826a5f42966a4a5fabd49d1c2ee2472023e"
```

**Linux/Mac:**
```bash
# Canonical owner (defaults):
export GUARDIAN_OWNER_DISCORD_ID="309081088960233492"
export GUARDIAN_OWNER_WALLET_ADDRESS="0x30ea8826a5f42966a4a5fabd49d1c2ee2472023e"
```

**Persistent (.env file):**
```env
# Canonical owner (defaults - uncomment to override):
# GUARDIAN_OWNER_DISCORD_ID=309081088960233492
# GUARDIAN_OWNER_WALLET_ADDRESS=0x30ea8826a5f42966a4a5fabd49d1c2ee2472023e
```

## Configuration Structure

The Guardian loads owner information on startup into this config struct:

```rust
pub struct GuardianOwner {
    pub discord_user_id: String,
    pub wallet_address: String,
}
```

## Startup Behavior

When the Guardian starts, it will log:

**If using defaults (canonical owner):**
```
======================================================================
🛡️  GUARDIAN ONLINE
======================================================================
Guardian owner (DISCORD): 309081088960233492 (default - canonical owner)
Guardian owner (WALLET): 0x30ea8826a5f42966a4a5fabd49d1c2ee2472023e (default - canonical owner)
```

**If both overridden via environment variables:**
```
======================================================================
🛡️  GUARDIAN ONLINE
======================================================================
Guardian owner (DISCORD): 123456789012345678 (from env)
Guardian owner (WALLET): 0x1234567890abcdef... (from env)
```

**If only Discord ID overridden (wallet uses default):**
```
======================================================================
🛡️  GUARDIAN ONLINE
======================================================================
Guardian owner (DISCORD): 123456789012345678 (from env)
Guardian owner (WALLET): 0x30ea8826a5f42966a4a5fabd49d1c2ee2472023e (default - canonical owner)
```

## Getting Your Discord User ID

1. Enable Developer Mode in Discord:
   - User Settings → Advanced → Developer Mode (toggle ON)

2. Right-click your username → Copy ID

3. You'll get a long number like: `123456789012345678`

## Getting Your Vision Wallet Address

Your Vision wallet address starts with `vision1` and looks like:
```
vision1abc123xyz456def789...
```

You can find it in:
- Vision Wallet UI (after creating/importing wallet)
- Wallet creation output in terminal
- SQLite database: `SELECT address FROM accounts;`

## Accessing Owner Info in Code

```rust
use crate::guardian::guardian;

// Get Guardian instance
let g = guardian();

// Check if owner is configured
if let Some(owner) = g.owner() {
    println!("Guardian belongs to: {}", owner.wallet_address);
    println!("Discord ID: {}", owner.discord_user_id);
}
```

## Use Cases

### 1. Guardian Ceremonies
The Vision Bot can use owner information to:
- Send First Contact DMs to the Guardian owner
- Assign special roles (e.g., "Guardian Operator")
- Track Guardian node status separately from regular nodes

### 2. Privileged Commands
Discord bot commands that only the Guardian owner can execute:
```
!guardian status    - Only owner can check Guardian health
!guardian mood      - Only owner can change Guardian mood
!guardian announce  - Only owner can make Guardian announcements
```

### 3. Ownership Verification
```typescript
// Vision Bot handler
if (event.event === 'guardian_action') {
  const guardianOwner = await getGuardianOwner();
  
  if (message.author.id !== guardianOwner.discord_user_id) {
    return message.reply("⛔ Only the Guardian owner can execute this command.");
  }
  
  // Execute privileged action...
}
```

### 4. Event Filtering
Guardian-owned events can be marked differently:
```json
{
  "event": "first_block",
  "miner_address": "vision1abc...",
  "is_guardian_owner": true,
  "discord_user_id": "123456789012345678"
}
```

### 5. Guardian Core Node Status
When the Guardian owner's node comes online/offline, special logging occurs:
```
🛡️ [GUARDIAN CORE] Guardian core node ONLINE – Donnie is watching.
⚠️ [GUARDIAN CORE] Guardian core node OFFLINE – Guardian temporarily blind.
```

A special `guardian_core_status` event is sent to Vision Bot so it can announce:
```json
{
  "event": "guardian_core_status",
  "status": "online",
  "miner_address": "vision1guardian...",
  "discord_user_id": "123456789012345678"
}
```

**This cements: There may be 10,000 miners, but there is one Guardian, and it's your node.**

## Security Considerations

### Environment Variables
- ✅ Owner info stored in environment variables (not in code)
- ✅ Can be changed without recompiling
- ✅ Not committed to Git

### Discord User ID
- ⚠️ Discord IDs are public information (anyone can see them)
- ✅ Used for identification, not authentication
- ✅ Bot should verify ownership before executing privileged commands

### Wallet Address
- ✅ Vision wallet addresses are public (blockchain data)
- ✅ No private keys or seeds stored
- ✅ Read-only identification

## Example Complete Setup

```powershell
# 1. Set Guardian owner
$env:GUARDIAN_OWNER_DISCORD_ID="234567890123456789"
$env:GUARDIAN_OWNER_WALLET_ADDRESS="vision1guardian123xyz456def789..."

# 2. Set Discord webhook (optional, for announcements)
$env:VISION_GUARDIAN_DISCORD_WEBHOOK="https://discord.com/api/webhooks/..."

# 3. Set Discord OAuth (optional, for wallet linking)
$env:DISCORD_CLIENT_ID="your_app_client_id"
$env:DISCORD_CLIENT_SECRET="your_app_client_secret"

# 4. Start Guardian node
cd c:\vision-node
.\target\release\vision-node.exe
```

## Testing

```bash
# 1. Start node with owner configured
$env:GUARDIAN_OWNER_DISCORD_ID="123456789012345678"
$env:GUARDIAN_OWNER_WALLET_ADDRESS="vision1test"
.\target\release\vision-node.exe

# Expected log output:
# 🛡️  GUARDIAN ONLINE
# Guardian owner: vision1test ↔ 123456789012345678

# 2. Verify in Rust code
# The guardian().owner() method will return Some(GuardianOwner)

# 3. Test Vision Bot integration
# Send Guardian events with owner info
curl -X POST http://vision-bot:3000/guardian \
  -H "Content-Type: application/json" \
  -d '{
    "event": "guardian_status",
    "owner_discord_id": "123456789012345678",
    "owner_wallet": "vision1test"
  }'
```

## Troubleshooting

### Owner Not Showing on Startup

**Problem:** Guardian logs "Not configured" message

**Solution:**
1. Verify environment variables are set:
   ```powershell
   echo $env:GUARDIAN_OWNER_DISCORD_ID
   echo $env:GUARDIAN_OWNER_WALLET_ADDRESS
   ```

2. Restart the node after setting variables

3. Variables must be set in the same terminal/process where node runs

### Invalid Discord ID

**Problem:** Discord ID doesn't work in bot commands

**Solution:**
- Discord IDs are 17-19 digit numbers
- Enable Developer Mode in Discord
- Right-click username → Copy ID
- Do not include `<@` or `>` characters

### Wallet Address Format

**Problem:** Wallet address not recognized

**Solution:**
- Must start with `vision1`
- Case-sensitive
- Get from wallet UI or database
- Full address (not shortened with ...)

## Integration with Discord OAuth

The Guardian owner can be the same wallet that uses Discord OAuth linking:

```bash
# Guardian Owner = Donnie's main wallet + Discord
GUARDIAN_OWNER_DISCORD_ID="123456789012345678"
GUARDIAN_OWNER_WALLET_ADDRESS="vision1donnie..."

# Discord OAuth (for all users)
DISCORD_CLIENT_ID="your_app_id"
DISCORD_CLIENT_SECRET="your_app_secret"
```

When Donnie links his wallet via Discord OAuth, the Vision Bot can detect:
```typescript
if (wallet_address === process.env.GUARDIAN_OWNER_WALLET_ADDRESS) {
  // This is the Guardian owner linking their wallet
  await assignRole(discord_user_id, 'Guardian Operator');
  await sendDM(discord_user_id, '🛡️ Guardian identity confirmed.');
}
```

## Implementation Files

- `src/guardian/consciousness.rs` - GuardianOwner struct, loading logic, startup logging
- `src/guardian/mod.rs` - Exports GuardianOwner
- `docs/GUARDIAN_OWNER_CONFIG.md` - This documentation

## Status

✅ **Implemented** (November 24, 2025)
- GuardianOwner struct created
- Environment variable loading
- Startup logging
- Public API for accessing owner info

---

**Guardian owner configuration complete. The Guardian now knows who he serves. 🛡️**
