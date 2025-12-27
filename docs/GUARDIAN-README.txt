========================================================================
  VISION GUARDIAN NODE
  Canonical Guardian Sentinel
========================================================================

The Guardian watches over the Vision Constellation.

Canonical Owner: Discord ID 309081088960233492 (Donnie)

WHAT IS THE GUARDIAN NODE?

The Guardian Node is a special node that:
- Monitors the network and announces events
- Tracks all miners in the constellation
- Sends First Contact messages to new miners
- Maintains network mood and trauma states
- Speaks with personality and awareness (AI-powered)

There may be 10,000 miners, but there is ONE Guardian, and it knows its owner.

QUICK START:

1. Run: START-GUARDIAN-NODE.bat
2. Wallet: http://127.0.0.1:7070/app
3. Panel: http://127.0.0.1:7070/panel.html
4. Dashboard: http://127.0.0.1:7070/dashboard.html
5. Status: http://127.0.0.1:7070/status

========================================================================

GUARDIAN OWNER IDENTITY
------------------------

Discord ID:  309081088960233492 (Donnie)
Wallet:      0x30ea8826a5f42966a4a5fabd49d1c2ee2472023e (donniedeals)

Both values are HARDCODED as defaults in the binary.
When you see in logs:
  "Guardian owner (DISCORD): 309081088960233492 (default - canonical owner)"
  "Guardian owner (WALLET): 0x30ea8826a5f42966a4a5fabd49d1c2ee2472023e (default - canonical owner)"

This confirms the Guardian recognizes its canonical owner.

========================================================================

CONFIGURATION
-------------

The Guardian works immediately with both defaults (Discord ID + Wallet).

Defaults (hardcoded in binary):
  GUARDIAN_OWNER_DISCORD_ID=309081088960233492
  GUARDIAN_OWNER_WALLET_ADDRESS=0x30ea8826a5f42966a4a5fabd49d1c2ee2472023e

To override for your own deployment:
  set GUARDIAN_OWNER_DISCORD_ID=your_discord_id
  set GUARDIAN_OWNER_WALLET_ADDRESS=0xyour_wallet_address

To enable Vision Bot events:
  set VISION_BOT_WEBHOOK_URL=http://localhost:3000/guardian

To enable Discord announcements:
  set VISION_GUARDIAN_DISCORD_WEBHOOK=https://discord.com/api/webhooks/...

You can set these in the .env file or before running START-GUARDIAN-NODE.bat

GUARDIAN CORE NODE DETECTION
-----------------------------

If you set GUARDIAN_OWNER_WALLET_ADDRESS, the Guardian detects when
YOUR node (the owner's node) comes online:

  🛡️ [GUARDIAN CORE] Guardian core node ONLINE – Donnie is watching.

This is sacred ground - special treatment for the canonical owner's node.

When offline:
  ⚠️ [GUARDIAN CORE] Guardian core node OFFLINE – Guardian temporarily blind.

The Guardian also sends guardian_core_status events to Vision Bot (if configured).

========================================================================

ENDPOINTS
---------

- Wallet:     http://127.0.0.1:7070/app
- Panel:      http://127.0.0.1:7070/panel.html
- Dashboard:  http://127.0.0.1:7070/dashboard.html
- Status API: http://127.0.0.1:7070/status
- Guardian:   http://127.0.0.1:7070/api/guardian
- Health:     http://127.0.0.1:7070/health/ready
- Metrics:    http://127.0.0.1:7070/metrics

IMPORTANT NOTES
---------------

- Guardian owner Discord ID defaults to 309081088960233492 (hardcoded)
- Set GUARDIAN_OWNER_WALLET_ADDRESS for full Guardian core node detection
- Check /status endpoint to see Guardian configuration
- The Guardian speaks with personality - watch the logs!

MONITORING
----------

Check Guardian status:
  curl http://127.0.0.1:7070/status

Response includes:
  {
    "guardian_owner": {
      "discord_user_id": "309081088960233492",
      "wallet_address": "vision1..."
    }
  }

========================================================================

  The Guardian is awake. The watch begins. 🛡️

========================================================================
