# P2P Peer Configuration Feature

## Overview
VisionNode v0.1.6 includes a new "Connect to Public Node" feature in the wallet Settings page that allows users to configure their local node to automatically connect to a public seed node for P2P gossip and synchronization.

## Features

### Backend Implementation
- **Configuration Persistence**: P2P peer address is saved to `vision_data_<port>/node_peer.json`
- **Auto-Load on Startup**: Node reads saved peer config and applies it automatically on startup
- **Node Restart**: Setting a new peer triggers an automatic node restart to apply changes
- **Admin Endpoint**: `POST /api/admin/node-peer` with JSON body `{"p2p_peer": "host:port"}`

### Frontend Implementation
- **Settings UI**: New "Connect to Public Node" section in Settings page
- **Input Validation**:
  - Must contain colon (`:`)
  - Cannot start with `http://` or `https://`
  - Cannot be empty
- **User Feedback**: Toast notifications and status messages
- **Instructions**: Helpful hints displayed below input field

## Usage

### For End Users
1. Open wallet and navigate to Settings (gear icon)
2. Scroll to "Connect to Public Node" section
3. Enter the public seed node address in format `host:port`
   - Example: `6.tcp.us-cal-1.ngrok.io:18527`
   - Example: `seed.example.com:7070`
4. Click "Apply & Restart" button
5. Wait a few seconds for node to restart
6. Reload the wallet page

### For Network Operators
To provide a public seed node for others to connect to:

1. **Using ngrok (temporary)**:
   ```bash
   # Start a TCP tunnel on your node's port
   ngrok tcp 7070
   
   # Share the forwarding address (e.g., 6.tcp.us-cal-1.ngrok.io:18527)
   ```

2. **Using Port Forwarding (permanent)**:
   - Configure your router to forward external port to your node's port
   - Share your public IP and port (e.g., `203.0.113.1:7070`)
   - Consider using a domain name with Dynamic DNS

## API Specification

### POST /api/admin/node-peer

**Request Body**:
```json
{
  "p2p_peer": "host:port"
}
```

**Success Response** (200):
```json
{
  "status": "restarting_local_node"
}
```

**Error Responses**:
- `400 Bad Request`: Invalid format (missing colon, includes http://, empty)
- `401 Unauthorized`: Missing or invalid admin token
- `500 Internal Server Error`: Failed to save configuration

## File Structure

### Backend (Rust)
- `src/main.rs`:
  - `NodePeerConfig` struct (line ~10910)
  - `load_node_peer_config()` function
  - `save_node_peer_config()` function
  - `update_node_peer()` endpoint handler (line ~25790)
  - Auto-load logic in `main()` (line ~3985)
  - Route registration (line ~5268)

### Frontend (TypeScript)
- `wallet-marketplace-source/src/routes/Settings.tsx`:
  - `handleApplyP2pPeer()` function
  - UI section with input and button

### Data Files
- `vision_data_<port>/node_peer.json`: Persisted peer configuration
  ```json
  {
    "p2p_peer": "host:port"
  }
  ```

## Testing

### Manual Test Steps
1. Start local node on default port (7070)
2. Open wallet at http://localhost:7070/app
3. Go to Settings → Connect to Public Node
4. Enter a test peer address (e.g., `test.example.com:7070`)
5. Click "Apply & Restart"
6. Verify toast notification appears
7. Wait for node to restart
8. Check logs for "Loading persisted P2P peer configuration"
9. Verify `vision_data_7070/node_peer.json` was created
10. Restart node manually and verify peer is auto-loaded

### Validation Tests
- Empty input → Error toast
- Missing colon → Error toast
- Input with `http://` → Error toast
- Valid input → Success, node restarts
- Persistence → Peer survives node restarts

## Security Considerations
- Endpoint requires admin authentication (localhost or VISION_ADMIN_TOKEN)
- No remote restart capability without proper auth
- User must have access to local machine to use feature
- Peer address is stored in plaintext (no sensitive data)

## Package Updates
- **Windows**: `VisionNode-v0.1.6-testnet2-WIN64.zip` (14.66 MB)
- **Linux**: `VisionNode-v0.1.6-testnet2-LINUX64.tar.gz` (4.28 MB)

Both packages include:
- Updated `vision-node` executable with P2P peer backend
- Updated wallet with Settings UI for P2P configuration
- All existing features from v0.1.6

## Troubleshooting

### Peer not connecting
- Verify peer address format is correct (host:port, no http://)
- Check if peer node is running and accessible
- Review node logs for connection errors
- Test peer connectivity with `telnet host port`

### Node not restarting
- Check if node has file write permissions for data directory
- Review terminal/logs for startup errors
- Manually restart node if auto-restart fails

### Configuration not persisting
- Verify `node_peer.json` exists in data directory
- Check file permissions on data directory
- Review logs for JSON parse errors

## Future Enhancements
- Support for multiple peer addresses
- Peer health monitoring in UI
- Auto-discovery of optimal peers
- Peer performance metrics
