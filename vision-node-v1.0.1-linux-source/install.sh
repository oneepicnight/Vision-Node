#!/bin/bash

# Vision Node v2.0.0 Installation Script (Constellation)

set -e

echo ""
echo "========================================"
echo "  VISION NODE v2.0.0 INSTALLER"
echo "  MODE: CONSTELLATION (SWARM ONLY)"
echo "========================================"
echo ""

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo " Error: Cargo.toml not found. Please run this script from the VisionNode directory."
    exit 1
fi

echo " Checking dependencies..."

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo " Rust/Cargo not found!"
    echo ""
    echo "Install Rust with:"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo ""
    exit 1
fi

echo " Rust found: $(rustc --version)"

# Check for required build dependencies
echo " Checking build dependencies..."

# Check for OpenSSL
if ! ldconfig -p | grep -q libssl.so 2>/dev/null && ! pkg-config --exists openssl 2>/dev/null; then
    echo "  OpenSSL not found - REQUIRED for build"
    echo ""
    echo "Install on Ubuntu/Debian:"
    echo "  sudo apt-get install libssl-dev pkg-config"
    echo ""
    echo "Install on Fedora/RHEL:"
    echo "  sudo dnf install openssl-devel"
    echo ""
    echo "Install on Arch:"
    echo "  sudo pacman -S openssl pkg-config"
    echo ""
    exit 1
fi

# Check for build essentials (gcc or clang required)
if ! command -v gcc &> /dev/null && ! command -v clang &> /dev/null; then
    echo "  No C compiler found (gcc/clang) - REQUIRED for build"
    echo ""
    echo "Install on Ubuntu/Debian:"
    echo "  sudo apt-get install build-essential clang lld"
    echo ""
    echo "Install on Fedora/RHEL:"
    echo "  sudo dnf groupinstall 'Development Tools'"
    echo "  sudo dnf install clang lld"
    echo ""
    echo "Install on Arch:"
    echo "  sudo pacman -S base-devel clang lld"
    echo ""
    exit 1
fi

# Check for pkg-config
if ! command -v pkg-config &> /dev/null; then
    echo "  pkg-config not found - REQUIRED for build"
    echo ""
    echo "Install on Ubuntu/Debian:"
    echo "  sudo apt-get install pkg-config"
    echo ""
    exit 1
fi

echo "  ✓ All build dependencies found"

echo ""
echo " Building Vision Node..."
echo ""

# Build the project
cargo build --release

if [ $? -eq 0 ]; then
    echo ""
    echo " Build successful!"
    echo ""
    
    # Make start script executable
    chmod +x start-node.sh 2>/dev/null || true
    
    # Create .env if it doesn't exist
    if [ ! -f ".env" ]; then
        if [ -f ".env.example" ]; then
            cp .env.example .env
            echo " Created .env from .env.example"
            echo "   Please edit .env and set your VISION_WALLET_ADDRESS"
        fi
    fi
    
    # Create data directory
    VISION_PORT="${VISION_PORT:-7070}"
    mkdir -p "./vision_data_${VISION_PORT}"
    echo " Created data directory: ./vision_data_${VISION_PORT}"
    
    echo ""
    echo "========================================"
    echo "  INSTALLATION COMPLETE!"
    echo "========================================"
    echo ""
    echo "Binary location: ./target/release/vision-node"
    echo ""
    echo "Next steps:"
    echo "  1. Edit .env and set your VISION_WALLET_ADDRESS"
    echo "  2. Optional: Enable UPnP with VISION_UPNP_ENABLED=true"
    echo "  3. Run: ./start-node.sh"
    echo ""
    echo "Or run directly:"
    echo "  ./target/release/vision-node"
    echo ""
    echo "Firewall setup (if needed):"
    echo "  sudo ufw allow 7072/tcp  # P2P port"
    echo "  sudo ufw allow 7070/tcp  # HTTP API (optional)"
    echo ""
    echo "Access points:"
    echo "  Wallet:    http://localhost:7070/app"
    echo "  Panel:     http://localhost:7070/panel.html"
    echo "  Dashboard: http://localhost:7070/dashboard.html"
    echo ""
else
    echo ""
    echo " Build failed!"
    echo ""
    exit 1
fi