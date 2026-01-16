# Vision Node v1.0 - Linux Source Build

## Requirements

- **Rust 1.70+** (Install from https://rustup.rs/)
- **Build Tools & Dependencies**
  
  **Ubuntu/Debian:**
  ```bash
  sudo apt-get update
  sudo apt-get install -y build-essential pkg-config libssl-dev clang lld
  ```
  
  **CentOS/RHEL:**
  ```bash
  sudo yum groupinstall 'Development Tools'
  sudo yum install -y pkg-config openssl-devel clang lld
  ```
  
  **Fedora:**
  ```bash
  sudo dnf groupinstall 'Development Tools'
  sudo dnf install -y pkg-config openssl-devel clang lld
  ```
  
  **Arch Linux:**
  ```bash
  sudo pacman -S base-devel openssl pkg-config clang lld
  ```

## Quick Start (Automated Installer)

```bash
# Extract the tarball
tar -xzf vision-node-v1.0-linux-source.tar.gz
cd vision-node-v1.0-linux-source

# Run the installer (handles dependencies, build, and setup)
chmod +x INSTALL_GUIDE.sh
./INSTALL_GUIDE.sh

# Or use the quick installer:
chmod +x install.sh
./install.sh
```

## Manual Build Instructions

```bash
# 1. Ensure Rust is installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 2. Install system dependencies (see Requirements section above)

# 3. Clean any previous builds (recommended)
cargo clean

# 4. Build the binary (takes 10-20 minutes)
cargo build --release

# 5. Binary will be at: ./target/release/vision-node
```

## Troubleshooting Build Issues

**Linker errors (undefined reference to `__rust_probestack`):**
- Install clang and lld: `sudo apt-get install clang lld`
- Update Rust: `rustup update stable`
- Clean and rebuild: `cargo clean && cargo build --release`

**OpenSSL not found:**
- Ubuntu/Debian: `sudo apt-get install libssl-dev pkg-config`
- CentOS/RHEL: `sudo yum install openssl-devel pkg-config`

**Out of memory during compilation:**
- Add swap space or build with fewer parallel jobs: `cargo build --release -j 2`

## Running the Node

```bash
# Start the node
./target/release/vision-node

# Or copy binary to current directory and run
cp ./target/release/vision-node ./
./vision-node
```

The node will:
- Create `vision_data_7070/` directory for blockchain data
- Start HTTP API on port 7070
- Start P2P network on port 7072
- Auto-generate `keys.json`, `miner.json`, `p2p.json` if not present
- Load system configuration from `config/token_accounts.toml` (required)

## Access the Wallet

Open your browser to: **http://localhost:7070**

## Configuration

**Required Files:**
- **config/token_accounts.toml** - System accounts and fee distribution (REQUIRED)

**Optional Files (auto-generated on first run):**
- **keys.json** - Wallet configuration
- **miner.json** - Mining settings (CPU threads, etc.)
- **p2p.json** - Network configuration (ports, peers)

## Firewall Setup

Allow incoming P2P connections:

**UFW (Ubuntu/Debian):**
```bash
sudo ufw allow 7070/tcp
sudo ufw allow 7072/tcp
sudo ufw reload
```

**Firewalld (CentOS/RHEL):**
```bash
sudo firewall-cmd --permanent --add-port=7070/tcp
sudo firewall-cmd --permanent --add-port=7072/tcp
sudo firewall-cmd --reload
```

**iptables:**
```bash
sudo iptables -A INPUT -p tcp --dport 7070 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 7072 -j ACCEPT
```

## Systemd Service (Optional)

Create `/etc/systemd/system/vision-node.service`:

```ini
[Unit]
Description=Vision Node
After=network.target

[Service]
Type=simple
User=YOUR_USERNAME
WorkingDirectory=/path/to/vision-node-v1.0-linux-source
ExecStart=/path/to/vision-node-v1.0-linux-source/target/release/vision-node
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable vision-node
sudo systemctl start vision-node
sudo systemctl status vision-node
```

## Troubleshooting

**Compilation fails with OpenSSL errors:**
- Install development libraries: `sudo apt install libssl-dev pkg-config`

**Port already in use:**
- Edit `p2p.json` to change ports

**Permission denied:**
- Make binary executable: `chmod +x ./target/release/vision-node`

## Support

- Website: https://visionworld.one
- Documentation: Check the `/docs` folder
- Community: Join our Discord/Telegram

---

**Version:** 1.0  
**Max Supply:** 1 Billion LAND  
**Consensus:** Proof of Work → Proof of Stake (at 1B supply)
