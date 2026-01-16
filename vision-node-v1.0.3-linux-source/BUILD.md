# VisionX Node v1.0.3 - Build Instructions

## Quick Start (Linux)

```bash
# 1. Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 2. Install system dependencies
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev

# 3. Build the node
cd vision-node-v1.0.3-linux-source
cargo build --release

# 4. Binary location
# target/release/vision-node
```

## System Requirements

**Minimum:**
- CPU: 2 cores
- RAM: 4 GB
- Disk: 20 GB free space
- OS: Ubuntu 20.04+ / Debian 11+

**Recommended:**
- CPU: 4+ cores
- RAM: 8 GB
- Disk: 50 GB SSD
- OS: Ubuntu 22.04 LTS

## Build Time

- **First build:** 10-15 minutes
- **Incremental:** 2-5 minutes

## Installation

```bash
# Option 1: System-wide installation
sudo cp target/release/vision-node /usr/local/bin/

# Option 2: User directory
mkdir -p ~/bin
cp target/release/vision-node ~/bin/
echo 'export PATH="$HOME/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Verify installation
vision-node --version
```

## Running the Node

```bash
# Basic start
vision-node

# With custom config
vision-node --config /path/to/config.json

# Check logs
tail -f ~/.visionx/logs/vision.log
```

## Troubleshooting

### "error: linker `cc` not found"
```bash
sudo apt install -y build-essential
```

### "error: failed to run custom build command for `openssl-sys`"
```bash
sudo apt install -y pkg-config libssl-dev
```

### Build takes too long
```bash
# Use faster linker (optional)
cargo install -f cargo-binutils
rustup component add llvm-tools-preview
```

## See CRITICAL_UPGRADE_v1.0.3.md for upgrade details
