# Discord OAuth - Wallet Team Integration Guide

## ✅ What's Already Done

**Backend (100% Complete):**
- ✅ Discord OAuth endpoints: `/api/discord/login`, `/api/discord/callback`, `/api/discord/status`
- ✅ SQLite database for wallet ↔ Discord mappings
- ✅ Guardian webhook integration (`link_wallet_discord` event)
- ✅ Environment variable support (DISCORD_CLIENT_ID, DISCORD_CLIENT_SECRET, DISCORD_REDIRECT_URI)
- ✅ HMAC-signed state tokens (10-minute expiry)
- ✅ Error handling and logging

**Frontend Components:**
- ✅ `LinkDiscordStep.tsx` - Fully functional Discord linking component

---

## 📝 TODO: Wallet Team Integration (2 tasks)

### Task 1: Add LinkDiscordStep to Wallet Creation Flow

**File:** `wallet-marketplace-source/src/pages/CreateWallet.tsx` (or wherever onboarding flow is)

**Current Flow:**
```
Welcome → Create Wallet → Show Seed → Backup Seed → Done (redirect to /app/home)
```

**New Flow:**
```
Welcome → Create Wallet → Show Seed → Backup Seed → **Link Discord (optional)** → Done
```

**Example Implementation:**

```tsx
import LinkDiscordStep from './LinkDiscordStep'

export default function CreateWallet() {
  const [step, setStep] = useState<'seed' | 'backup' | 'discord' | 'done'>('seed')
  const [walletAddress, setWalletAddress] = useState('')
  const navigate = useNavigate()

  // ... existing seed generation logic ...

  if (step === 'discord') {
    return (
      <LinkDiscordStep
        walletAddress={walletAddress}
        onSkip={() => {
          // User chose to skip Discord linking
          setStep('done')
          navigate('/app/home')
        }}
        onLinked={() => {
          // User successfully linked Discord (or was already linked)
          setStep('done')
          navigate('/app/home')
        }}
      />
    )
  }

  // ... rest of onboarding steps ...
}
```

---

### Task 2: Add LinkDiscordStep to Import Wallet Flow

**File:** `wallet-marketplace-source/src/pages/ImportWalletScreen.tsx`

**Current Flow:**
```
Import Screen → Enter Seed → Import Success → Redirect to /app/home
```

**New Flow:**
```
Import Screen → Enter Seed → Import Success → **Link Discord (optional)** → Redirect to /app/home
```

**Example Implementation:**

```tsx
import LinkDiscordStep from './LinkDiscordStep'

export default function ImportWalletScreen() {
  const [showDiscordStep, setShowDiscordStep] = useState(false)
  const [importedAddress, setImportedAddress] = useState('')
  const navigate = useNavigate()

  const handleImport = () => {
    // ... existing import logic ...
    
    // After successful import:
    setImportedAddress(walletAddress)
    setShowDiscordStep(true)
  }

  if (showDiscordStep) {
    return (
      <LinkDiscordStep
        walletAddress={importedAddress}
        onSkip={() => {
          navigate('/app/home')
        }}
        onLinked={() => {
          navigate('/app/home')
        }}
      />
    )
  }

  // ... existing import UI ...
}
```

---

## 🧪 Testing Checklist

### Local Testing (Without Real Discord OAuth)

**Test Status Endpoint:**
```bash
# Should return { "linked": false }
curl "http://127.0.0.1:7070/api/discord/status?wallet_address=vision1test123"
```

**Test Login Endpoint:**
```bash
# Should return { "url": "https://discord.com/api/oauth2/authorize..." }
curl "http://127.0.0.1:7070/api/discord/login?wallet_address=vision1test123"
```

### Full OAuth Testing (Requires Discord App)

**Setup:**
1. Create Discord app at https://discord.com/developers/applications
2. Add redirect URI: `http://127.0.0.1:7070/api/discord/callback`
3. Set environment variables:
   ```bash
   $env:DISCORD_CLIENT_ID="your_client_id"
   $env:DISCORD_CLIENT_SECRET="your_client_secret"
   ```
4. Restart Vision Node

**Test Flow:**
1. Create new wallet in UI
2. Reach Discord linking step
3. Click "LINK DISCORD"
4. Authorize on Discord
5. Should redirect back to wallet
6. Component should show "✅ Linked to @username"
7. Should auto-continue to wallet home after 2 seconds

**Verify Backend:**
```bash
# Check SQLite database
sqlite3 vision_discord_links.db "SELECT * FROM discord_links;"
```

**Expected Guardian Event:**
```json
{
  "event": "link_wallet_discord",
  "wallet_address": "vision1abc123...",
  "discord_user_id": "123456789012345678",
  "discord_username": "username#1234"
}
```

---

## 🎨 UI Behavior Notes

### LinkDiscordStep Component Features:

1. **Auto-detection:** Checks Discord status on mount
2. **Already linked:** Auto-continues to wallet home after 2 seconds
3. **Skip option:** Always available via "Skip for now" link
4. **Loading states:** Shows loading spinner during API calls
5. **Error handling:** Displays error messages in red box
6. **Address shortening:** Shows `vision1abc...xyz123` format
7. **Status indicator:** ✅ green for linked, ❌ gray for not linked

### Styling:

Component uses existing `wallet-aaa.css` classes:
- `.vision-landing` - Full-screen background
- `.vision-landing--inner` - Inner container with spinning globe
- `.import-panel` - Centered content panel
- `.vision-title-sub-small` - Large gradient title
- `.btn-primary` - Primary action button
- Inline styles for status card and error box

---

## 🔗 Integration Points Summary

**Add to onboarding:** After seed backup confirmation
**Add to import:** After successful seed import
**Component:** `<LinkDiscordStep walletAddress={addr} onSkip={fn} onLinked={fn} />`
**Props:** 3 required props (walletAddress, onSkip, onLinked)
**Duration:** ~1 minute to integrate each flow

---

## 📚 Related Documentation

- Full docs: `docs/DISCORD_OAUTH_INTEGRATION.md`
- Guardian webhooks: `docs/GUARDIAN_EVENT_WEBHOOKS.md`
- Quick reference: `docs/GUARDIAN_WEBHOOK_QUICKSTART.md`

---

**Status:** ⏳ PENDING WALLET TEAM INTEGRATION (2 simple additions)
**Component:** ✅ READY TO USE
**Backend:** ✅ FULLY OPERATIONAL
**Estimated Time:** 30 minutes total
