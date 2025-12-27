# Vision Wallet - Troubleshooting 404 Errors

## Understanding How The Wallet Works

**IMPORTANT**: The Vision Wallet is **NOT** a standalone application. It's a web interface that is **served by the Vision Node**.

```
Vision Node (vision-node.exe)
  └── Built-in Web Server (port 7070)
       ├── /api/* ................... JSON API endpoints
       ├── /panel.html .............. Miner control panel
       └── /wallet/ ................. Web wallet (static files)
            ├── index.html
            ├── assets/
            └── vite.svg
```

## Why You Get 404 Errors

A 404 error on `http://localhost:7070/wallet/` means ONE of these:

### 1. Vision Node Is NOT Running ❌
   - **The wallet cannot work if the node isn't running**
   - The node provides the web server that serves the wallet files
   - **FIX**: Start the node by double-clicking the "Vision Node" desktop icon

### 2. Wallet Files Not Installed ❌
   - The wallet ZIP was downloaded but installer wasn't run
   - Files should be in: `%LOCALAPPDATA%\VisionBlockchain\public\wallet\`
   - **FIX**: Run `INSTALL.bat` from the VisionWallet-v1.0.zip

### 3. Vision Node Not Installed ❌
   - You can't install the wallet without the node
   - The wallet installs INTO the node's directory structure
   - **FIX**: Install Vision Node first (VisionNode-v1.0.zip)

## Step-by-Step Installation (Correct Order)

### Step 1: Install Vision Node
```
1. Extract VisionNode-v1.0.zip
2. Double-click INSTALL.bat
3. Wait for "Installation Complete"
4. You'll get 2 desktop icons: "Vision Node" and "Vision Miner"
```

### Step 2: Start Vision Node
```
1. Double-click "Vision Node" desktop icon
2. A black terminal window will open
3. You'll see "Mining initialized" messages
4. Leave this window OPEN (minimized is fine)
```

### Step 3: Install Vision Wallet
```
1. Extract VisionWallet-v1.0.zip
2. Double-click INSTALL.bat
3. Wait for "Installation Complete"
4. You'll get a new icon: "Vision Wallet"
```

### Step 4: Use The Wallet
```
1. Make sure the node is still running (terminal window open)
2. Double-click "Vision Wallet" desktop icon
3. Browser opens to: http://localhost:7070/wallet/
4. Wallet loads (no 404!)
```

## Quick Diagnostic

Run the diagnostic script to check your setup:
```
diagnose-wallet.bat
```

This will tell you exactly what's wrong:
- ✅ Node installed?
- ✅ Node running?
- ✅ Wallet installed?
- ✅ Wallet accessible?

## Common Mistakes

### ❌ "I installed the wallet but it doesn't work"
- Did you install the node first?
- Is the node currently running?
- The wallet needs BOTH: installed + node running

### ❌ "I closed the terminal window and now wallet is 404"
- That terminal IS the node running
- Closing it stops the web server
- Solution: Start the node again

### ❌ "I only see panel.html working, not wallet"
- The wallet installer wasn't run
- The panel is included with the node by default
- The wallet is a separate optional install

### ❌ "Can't I just run 'npm run dev' in the wallet folder?"
- The wallet is already BUILT (compiled to static files)
- You don't need npm, node, or Vite
- Just install it and let the Vision Node serve it

## Manual Installation Check

If you want to verify manually:

### Check Node Installation:
```powershell
dir %LOCALAPPDATA%\VisionBlockchain
```
Should see: `vision-node.exe`, `config/`, `public/`

### Check Wallet Installation:
```powershell
dir %LOCALAPPDATA%\VisionBlockchain\public\wallet
```
Should see: `index.html`, `vite.svg`, `assets/`

### Check Node Running:
```powershell
curl http://localhost:7070/api/admin/ping
```
Should return: `{"message":"pong",...}`

### Check Wallet Loading:
```powershell
curl http://localhost:7070/wallet/
```
Should return: HTML content (not 404)

## Architecture Diagram

```
┌─────────────────────────────────────────────────────┐
│  Your Computer                                      │
│                                                     │
│  ┌──────────────────────────────────────────────┐ │
│  │  Vision Node (vision-node.exe)               │ │
│  │  Listening on http://localhost:7070          │ │
│  │                                              │ │
│  │  ┌────────────────────────────────────────┐ │ │
│  │  │  Built-in Axum Web Server              │ │ │
│  │  │                                        │ │ │
│  │  │  Serves files from: public/           │ │ │
│  │  │  ├── panel.html (miner UI)            │ │ │
│  │  │  └── wallet/                          │ │ │
│  │  │       ├── index.html                  │ │ │
│  │  │       ├── assets/index-*.js           │ │ │
│  │  │       └── vite.svg                    │ │ │
│  │  └────────────────────────────────────────┘ │ │
│  └──────────────────────────────────────────────┘ │
│                       ▲                            │
│                       │                            │
│                       │ HTTP Request               │
│                       │                            │
│  ┌──────────────────────────────────────────────┐ │
│  │  Web Browser                                 │ │
│  │  http://localhost:7070/wallet/               │ │
│  └──────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

## Still Having Issues?

1. Run `diagnose-wallet.bat` and read the output carefully
2. Make sure you followed the installation order (Node THEN Wallet)
3. Verify the node is running (terminal window open with mining logs)
4. Try opening `http://localhost:7070/panel.html` first
   - If panel works but wallet doesn't → wallet not installed
   - If panel doesn't work → node not running
5. Check Windows Firewall isn't blocking port 7070
6. Try accessing from a different browser

## The Vite Build Confusion

You might wonder: "The wallet source uses Vite, why doesn't it run with `npm run dev`?"

**Answer**: The wallet WAS built with Vite, creating static files:
- `wallet-marketplace-source/` ← Development source code (uses Vite)
- `wallet-final/` ← Production build (static HTML/JS/CSS)
- `VisionWallet-v1.0.zip` ← The built files, ready to deploy

When you install the wallet, you're copying the **already-built** static files.
The Vision Node's web server serves them like any other static website.

No separate Vite server needed. No npm. No Node.js.
Just the Vision Node binary serving static files.
