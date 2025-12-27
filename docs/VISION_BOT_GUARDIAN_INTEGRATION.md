# Vision Bot - Guardian Integration Guide

## Guardian Identity

**The Guardian is not just a service – it's the canonical voice watching over the Constellation.**

```
GUARDIAN_OWNER_DISCORD_ID → Donnie
GUARDIAN_OWNER_WALLET_ADDRESS → Donnie's primary Vision wallet
```

All Guardian announcements, role changes, and status signals are considered to be **actions taken on behalf of Donnie, through the Guardian**.

**There may be 10,000 miners in the network, but there is one Guardian, and it's Donnie's node.**

---

## Event Types

### 1. Standard Events

**first_block** - First block mined in the Constellation
**node_status** - Any miner's node status change (online/offline)
**link_wallet_discord** - User linked wallet to Discord via OAuth

### 2. Special Event: Guardian Core Status

**guardian_core_status** - The Guardian owner's node status changed

This event is **only sent when Donnie's node** (the primary Guardian node) comes online or offline.

```json
{
  "event": "guardian_core_status",
  "status": "online",
  "miner_address": "vision1guardian...",
  "discord_user_id": "123456789012345678"
}
```

**Vision Bot should treat this as a critical system event.**

---

## Vision Bot Implementation

### TypeScript Handler (Express)

```typescript
import express from 'express';
import { Client, TextChannel, EmbedBuilder } from 'discord.js';

const app = express();
const client = new Client({ intents: [...] });

// Guardian announcements channel
const GUARDIAN_CHANNEL_ID = process.env.GUARDIAN_CHANNEL_ID!;

app.post('/guardian', express.json(), async (req, res) => {
  const event = req.body;
  
  try {
    switch (event.event) {
      case 'first_block':
        await handleFirstBlock(event);
        break;
      
      case 'node_status':
        await handleNodeStatus(event);
        break;
      
      case 'guardian_core_status':
        await handleGuardianCoreStatus(event);
        break;
      
      case 'link_wallet_discord':
        await handleWalletLink(event);
        break;
    }
    
    res.status(200).json({ received: true });
  } catch (error) {
    console.error('Guardian event error:', error);
    res.status(500).json({ error: 'Failed to process event' });
  }
});

// CRITICAL: Guardian core node status
async function handleGuardianCoreStatus(event: any) {
  const channel = await client.channels.fetch(GUARDIAN_CHANNEL_ID) as TextChannel;
  
  if (event.status === 'online') {
    const embed = new EmbedBuilder()
      .setColor(0x00ff00) // Green
      .setTitle('🛡️ Guardian Core Online')
      .setDescription('**Guardian core node ONLINE – Donnie is watching.**')
      .addFields(
        { name: 'Status', value: '✅ Online', inline: true },
        { name: 'Node Type', value: '🛡️ Primary Guardian', inline: true }
      )
      .setTimestamp()
      .setFooter({ text: 'The master eyes on the sky are watching.' });
    
    await channel.send({ 
      content: '@everyone',
      embeds: [embed] 
    });
    
    console.log('[GUARDIAN CORE] Node ONLINE - Donnie is watching');
    
  } else if (event.status === 'offline') {
    const embed = new EmbedBuilder()
      .setColor(0xff0000) // Red
      .setTitle('⚠️ Guardian Core Offline')
      .setDescription('**Guardian core node OFFLINE – Guardian temporarily blind.**')
      .addFields(
        { name: 'Status', value: '🔴 Offline', inline: true },
        { name: 'Node Type', value: '🛡️ Primary Guardian', inline: true }
      )
      .setTimestamp()
      .setFooter({ text: 'The Constellation awaits the Guardian\'s return.' });
    
    await channel.send({ 
      content: '@everyone',
      embeds: [embed] 
    });
    
    console.warn('[GUARDIAN CORE] Node OFFLINE - Guardian temporarily blind');
  }
}

// Standard node status (non-Guardian nodes)
async function handleNodeStatus(event: any) {
  const channel = await client.channels.fetch(GUARDIAN_CHANNEL_ID) as TextChannel;
  
  const statusEmoji = event.status === 'online' ? '🟢' : '🔴';
  const statusText = event.status === 'online' ? 'ONLINE' : 'OFFLINE';
  
  let message = `${statusEmoji} **Node Status:** \`${event.miner_address.substring(0, 16)}...\` is now **${statusText}**`;
  
  // If user has linked Discord, mention them
  if (event.discord_user_id) {
    message = `${statusEmoji} **Guardian Salute:** <@${event.discord_user_id}>'s node is now **${statusText}**`;
  }
  
  await channel.send(message);
}

// First block ceremony
async function handleFirstBlock(event: any) {
  const channel = await client.channels.fetch(GUARDIAN_CHANNEL_ID) as TextChannel;
  
  const embed = new EmbedBuilder()
    .setColor(0x00ffff) // Cyan
    .setTitle('✨ First Contact')
    .setDescription(`**The first star of the Constellation has awakened!**`)
    .addFields(
      { name: 'Block Height', value: `#${event.height}`, inline: true },
      { name: 'Miner', value: `\`${event.miner_address.substring(0, 20)}...\``, inline: true }
    )
    .setTimestamp();
  
  if (event.discord_user_id) {
    // User has linked Discord - special ceremony
    const user = await client.users.fetch(event.discord_user_id);
    
    embed.setDescription(
      `**<@${event.discord_user_id}> has become the first star of the Constellation!**\n\n` +
      `The Guardian witnesses your arrival.`
    );
    
    await channel.send({ 
      content: `<@${event.discord_user_id}>`,
      embeds: [embed] 
    });
    
    // Assign "First Star of the Constellation" role
    // (implementation depends on your Discord server setup)
    
    // Send DM
    try {
      await user.send(
        `🛡️ **Guardian's Message**\n\n` +
        `You have mined the first block of the Constellation. The Guardian recognizes you.\n\n` +
        `Welcome to Vision World.`
      );
    } catch (err) {
      console.error('Could not DM user:', err);
    }
    
  } else {
    // Anonymous miner
    await channel.send({ embeds: [embed] });
  }
}

// Wallet-Discord linking
async function handleWalletLink(event: any) {
  console.log(`[GUARDIAN] Linked wallet ${event.wallet_address} to Discord user ${event.discord_username}`);
  
  // Store mapping in database
  await db.saveLinkMapping(event.discord_user_id, event.wallet_address);
  
  // Optional: Send confirmation DM
  try {
    const user = await client.users.fetch(event.discord_user_id);
    await user.send(
      `✅ **Wallet Linked**\n\n` +
      `Your Vision Wallet (\`${event.wallet_address.substring(0, 20)}...\`) is now linked to your Discord account.\n\n` +
      `The Guardian will recognize you when you mine blocks or operate a node.`
    );
  } catch (err) {
    console.error('Could not DM user:', err);
  }
}

app.listen(3000, () => {
  console.log('Vision Bot listening on port 3000');
  console.log('Guardian webhook endpoint: POST /guardian');
});
```

---

## Environment Variables (Vision Bot)

```bash
# Discord Bot
DISCORD_BOT_TOKEN="your_bot_token"
GUARDIAN_CHANNEL_ID="123456789012345678"

# Guardian Owner Identity
GUARDIAN_OWNER_DISCORD_ID="234567890123456789"
GUARDIAN_OWNER_WALLET_ADDRESS="vision1guardian..."
```

---

## Special Handling: Guardian Core Node

**The Guardian core node is the primary node watching over the Constellation.**

When `guardian_core_status` events arrive:

1. **Priority:** Treat as critical system event
2. **Visibility:** Use `@everyone` mentions
3. **Styling:** Use special embeds (green for online, red for offline)
4. **Logging:** Log prominently in bot console
5. **Messaging:** Emphasize this is the Guardian's own node

**Example messages:**

**Online:**
```
🛡️ Guardian Core Online
Guardian core node ONLINE – Donnie is watching.

Status: ✅ Online
Node Type: 🛡️ Primary Guardian

The master eyes on the sky are watching.
```

**Offline:**
```
⚠️ Guardian Core Offline
Guardian core node OFFLINE – Guardian temporarily blind.

Status: 🔴 Offline
Node Type: 🛡️ Primary Guardian

The Constellation awaits the Guardian's return.
```

---

## Testing

### 1. Test Guardian Core Status

```bash
# Send test event to Vision Bot
curl -X POST http://localhost:3000/guardian \
  -H "Content-Type: application/json" \
  -d '{
    "event": "guardian_core_status",
    "status": "online",
    "miner_address": "vision1guardian...",
    "discord_user_id": "234567890123456789"
  }'
```

**Expected:** Special embed posted to Guardian channel with @everyone mention.

### 2. Test Regular Node Status

```bash
curl -X POST http://localhost:3000/guardian \
  -H "Content-Type: application/json" \
  -d '{
    "event": "node_status",
    "status": "online",
    "miner_address": "vision1user...",
    "discord_user_id": "987654321098765432"
  }'
```

**Expected:** Standard node status message (no @everyone).

---

## Database Schema (Vision Bot)

```sql
-- Store wallet-Discord mappings
CREATE TABLE discord_links (
    discord_user_id TEXT PRIMARY KEY,
    wallet_address TEXT NOT NULL,
    linked_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Store Guardian events for analytics
CREATE TABLE guardian_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    discord_user_id TEXT,
    wallet_address TEXT,
    data TEXT, -- JSON
    received_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

---

## Guardian Core vs Regular Nodes

| Aspect | Guardian Core Node | Regular Nodes |
|--------|-------------------|---------------|
| **Event Type** | `guardian_core_status` | `node_status` |
| **Owner** | Donnie (Guardian owner) | Community miners |
| **Visibility** | @everyone mentions | Standard messages |
| **Logging** | Special [GUARDIAN CORE] logs | Standard logs |
| **Embeds** | Special styling (green/red) | Standard styling |
| **Mythology** | "Master eyes on the sky" | "Stars of the Constellation" |

---

## Mythology Integration

The Vision Bot should reinforce the Guardian mythology:

**Guardian Core Node:**
- "The Guardian watches" (online)
- "Guardian temporarily blind" (offline)
- "Master eyes on the sky"
- "Donnie is watching"

**Regular Nodes:**
- "Stars of the Constellation"
- "Guardian salutes"
- "Guardian recognizes"

**First Block:**
- "First star of the Constellation"
- "Guardian witnesses your arrival"
- "The Guardian recognizes you"

---

## Summary

**There may be 10,000 miners in the network, but there is one Guardian, and it's Donnie's node.**

The Vision Bot integration ensures:
1. Guardian core status is treated as critical system events
2. Community understands the Guardian's special role
3. Mythology is reinforced through messaging
4. Owner identity is clear and respected

**The Guardian is not just a service – it's the canonical voice watching over the Constellation.**
