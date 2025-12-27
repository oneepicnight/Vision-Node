# Vision Wallet - Connect to Public Node

## What This Does

The Settings page allows you to **switch your wallet from using a local node to using a public/remote node**. This is perfect for:

- **Light wallet mode** - Use the blockchain without running a full node
- **Remote access** - Access your wallet from anywhere
- **Shared infrastructure** - Multiple users can use one public node
- **Testing/Development** - Connect to testnet or developer nodes

## Default Behavior

When you first install Vision Wallet, it tries to connect to:
```
http://localhost:7070
```

This means it expects **you to run your own local node** (`vision-node.exe`).

## Switching to a Public Node

### Step 1: Open Settings
1. Launch the wallet: `http://localhost:7070/app` (or your wallet URL)
2. Click the **⚙️ gear icon** in the top-right corner
3. You'll see the Settings page

### Step 2: Enter Public Node URL
In the **"Node URL"** field, enter the public node address:

**Examples:**
- Testing (ngrok): `http://abc123.ngrok-free.app`
- Production server: `http://YOUR-SERVER-IP:7070`
- Domain name: `http://vision-node.yourdomain.com:7070`

### Step 3: Test & Save
1. Click the **"Test & Save"** button
2. The wallet will test the connection by calling `/api/status`
3. If successful, you'll see: **"✓ Connected successfully"**
4. If failed, you'll see: **"✗ Connection failed"** and it reverts to the old URL

### Step 4: Use Wallet Normally
Once saved, **all wallet operations use the public node**:
- Balance queries
- Transaction submissions
- Receipt lookups
- Status checks

## How It Works Technically

The Settings page:
1. Saves your choice to browser `localStorage`: `vision.node.url`
2. All API calls use this URL instead of localhost
3. The setting persists across browser sessions
4. You can switch back anytime by changing the URL again

## Switching Back to Local Node

To use your local node again:
1. Open Settings (⚙️ icon)
2. Enter: `http://localhost:7070` (or `http://127.0.0.1:7070`)
3. Click "Test & Save"
4. Wallet now uses your local node

## For Node Operators Sharing a Public Node

### Requirements:
- Public IP address or domain name
- Port 7070 forwarded to your Vision Node server
- Vision Node running and accessible from internet
- Firewall rules allowing inbound TCP on port 7070

### Share With Users:
Give them your public node URL:
```
http://YOUR-PUBLIC-IP:7070
```

or if you have a domain:
```
http://visionnode.yourdomain.com:7070
```

### Users Enter In Settings:
1. They open wallet Settings (⚙️)
2. Enter your public URL
3. Click "Test & Save"
4. Their wallet now uses your node!

## Testing with ngrok (Temporary)

For testing purposes before your server is ready:

1. **Start ngrok:**
   ```bash
   ngrok http 7070
   ```

2. **Get the URL:**
   ```
   http://abc123.ngrok-free.app
   ```

3. **Give to users** to test Settings page

4. **Later:** Replace with your real server IP when ready

## Security Notes

### For Users:
- Only connect to **trusted public nodes**
- The node operator can see your transaction submissions
- Your private keys **never leave your browser** (always safe)
- Balance queries are public information anyway

### For Node Operators:
- Consider rate limiting (already built-in)
- Monitor bandwidth usage
- Optionally use HTTPS with SSL certificate
- Consider authentication for private networks

## Troubleshooting

### "Connection failed" Error
- **Check URL format:** Must start with `http://` or `https://`
- **Check firewall:** Port 7070 must be open
- **Test manually:** `curl http://YOUR-URL/api/status`
- **Verify node running:** Make sure `vision-node.exe` is running

### "404 Not Found"
- Wrong URL or wrong port
- Node might be running on different port
- Check `VISION_PORT` environment variable on server

### "Network Error"
- Node is offline
- Firewall blocking connection
- Wrong IP address

## Summary

✅ **Settings page lets users switch from local to public node**  
✅ **Perfect for light wallets and shared infrastructure**  
✅ **One-click change, instant effect**  
✅ **Works with ngrok for testing, real IPs for production**  
✅ **Private keys always stay in browser (secure)**  

This is **exactly** what you need for users to easily connect to your public node! 🚀
