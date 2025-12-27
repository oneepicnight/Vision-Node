VISION NODE v2.0.0 - CONSTELLATION
===================================

QUICK START
-----------

1. Run the installer:
   ./install-linux.sh

2. Start the node:
   ./START-VISION-NODE-LINUX.sh

3. Access the web interface:
   http://localhost:7070

4. Check node health:
   curl http://localhost:7070/health
   curl http://localhost:7070/constellation/status


WHAT IS CONSTELLATION MODE?
----------------------------

Constellation v2.0.0 operates in "SwarmOnly" mode - a decentralized P2P network
that does NOT require any Guardian or Beacon nodes. All nodes are equal peers
that discover each other through:

- Hardcoded seed peers (7 testnet IPs)
- Peer-to-peer gossip protocol
- Automatic UPnP port forwarding (if enabled)


SYSTEM REQUIREMENTS
-------------------

- Linux x86_64 (Ubuntu 20.04+, Debian 11+, CentOS 8+, RHEL 8+)
- 2 GB RAM minimum, 4 GB recommended
- 10 GB disk space for blockchain data
- Internet connection
- Open ports: 7070 (HTTP) and 7072 (P2P)


FIREWALL CONFIGURATION
----------------------

Your firewall must allow incoming connections on both ports:

Ubuntu/Debian (UFW):
  sudo ufw allow 7070/tcp
  sudo ufw allow 7072/tcp
  sudo ufw reload

CentOS/RHEL (firewalld):
  sudo firewall-cmd --permanent --add-port=7070/tcp
  sudo firewall-cmd --permanent --add-port=7072/tcp
  sudo firewall-cmd --reload

iptables:
  sudo iptables -A INPUT -p tcp --dport 7070 -j ACCEPT
  sudo iptables -A INPUT -p tcp --dport 7072 -j ACCEPT


CONFIGURATION
-------------

Edit the .env file to customize your node:

VISION_PORT=7070              # HTTP API and web interface port
VISION_P2P_PORT=7072          # P2P mining and peer communication port
VISION_IPV4_ONLY=true         # Use IPv4 only (recommended)
VISION_UPNP_ENABLED=true      # Automatic port forwarding via UPnP

VISION_MIN_PEERS_FOR_MINING=2 # Minimum peers required to start mining
VISION_MAX_PEERS=50           # Maximum concurrent peer connections


UPNP PORT FORWARDING
--------------------

UPnP is enabled by default and will automatically configure your router to
forward port 7072 to your node. This allows other peers to connect to you.

- Lease Duration: 24 hours
- Renewal: Every 12 hours
- Auto-cleanup: On node shutdown

If UPnP is not available on your router, you must manually forward ports:
- Forward external port 7072 to your machine's local IP on port 7072


API ENDPOINTS
-------------

Health & Status:
  GET /health                    - Basic health check
  GET /constellation/status      - Full P2P network status
  GET /p2p/peers                 - List of connected peers
  GET /p2p/health                - P2P health metrics

Blockchain:
  GET /chain/tip                 - Current blockchain tip
  GET /chain/height              - Current block height
  GET /block/:height             - Get block by height
  GET /block/hash/:hash          - Get block by hash

Wallet & Mining:
  GET /wallet                    - Wallet information
  POST /miner/start              - Start mining
  POST /miner/stop               - Stop mining


NETWORK BOOTSTRAP
-----------------

Your node will automatically connect to 7 hardcoded seed peers:
  16.163.123.221:7072
  69.173.206.211:7072
  69.173.207.135:7072
  74.125.212.204:7072
  75.128.156.69:7072
  98.97.137.74:7072
  182.106.66.15:7072

The node will retry indefinitely until at least one peer is reachable.
Seeds are NEVER blacklisted in SwarmOnly mode.


RUNNING AS A SERVICE
--------------------

To run the node as a systemd service:

1. Create service file:
   sudo nano /etc/systemd/system/vision-node.service

2. Add this configuration:

   [Unit]
   Description=Vision Node v2.0.0 Constellation
   After=network.target

   [Service]
   Type=simple
   User=YOUR_USERNAME
   WorkingDirectory=/path/to/vision-node
   ExecStart=/path/to/vision-node/vision-node
   Restart=always
   RestartSec=10

   [Install]
   WantedBy=multi-user.target

3. Enable and start:
   sudo systemctl daemon-reload
   sudo systemctl enable vision-node
   sudo systemctl start vision-node

4. Check status:
   sudo systemctl status vision-node
   sudo journalctl -u vision-node -f


TROUBLESHOOTING
---------------

Node shows "NETWORK ISOLATED":
  - Check firewall allows port 7072 inbound
  - Verify router port forwarding (if UPnP disabled)
  - Wait for seed peers to come online
  - Check logs for connection attempts

Cannot access web interface:
  - Verify node is running: curl http://localhost:7070/health
  - Check firewall allows port 7070
  - Try accessing from the machine itself first

Mining not starting:
  - Check minimum peers requirement: curl http://localhost:7070/constellation/status
  - Ensure at least 2 peers connected before mining
  - Check miner endpoint: curl http://localhost:7070/miner/status

Binary won't execute:
  - Verify executable permissions: chmod +x vision-node
  - Check architecture: file vision-node (should show x86-64)
  - Install missing libraries: see install-linux.sh output


MONITORING
----------

Check node status:
  curl http://localhost:7070/constellation/status | jq

Watch peer connections:
  watch -n 5 'curl -s http://localhost:7070/p2p/peers | jq ".total_peers"'

Monitor mining:
  curl http://localhost:7070/miner/status

View P2P health:
  curl http://localhost:7070/p2p/health | jq


SECURITY
--------

- Never expose your wallet private key
- Use firewall to restrict access to port 7070 if public
- Consider running behind a reverse proxy (nginx, caddy)
- Regularly backup wallet/ directory
- Keep the .env file secure (contains node configuration)


SUPPORT
-------

For issues, questions, or updates:
- Check the logs in ./vision_data_7070/
- Review this README
- Verify all prerequisites are met
- Ensure ports are open and accessible


VERSION INFORMATION
-------------------

Version: v2.0.0
Codename: Constellation
Network: Testnet
Protocol: v2
Build: LINUX64-x86_64-musl
