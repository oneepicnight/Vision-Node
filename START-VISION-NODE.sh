#!/usr/bin/env bash
#
# Vision Node v2.0.0 - Constellation Mode Launcher
# ================================================

clear

echo "â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”"
echo "   ðŸŒŒ VISION NODE v2.0.0 - CONSTELLATION ðŸŒŒ"
echo "â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”"
echo ""
echo "  Mode: SwarmOnly (No Guardian/Beacon Required)"
echo "  HTTP Port: 7070"
echo "  P2P Port: 7072"
echo "  UPnP: Enabled (auto port-forward)"
echo ""
echo "â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”"
echo ""

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Set environment variables for Constellation mode
export VISION_PUBLIC_DIR="$SCRIPT_DIR/public"
export VISION_WALLET_DIR="$SCRIPT_DIR/wallet"
export RUST_BACKTRACE="${RUST_BACKTRACE:-0}"

# Load .env file if it exists
if [ -f ".env" ]; then
    export $(grep -v '^#' .env | xargs)
    echo "âœ“ Loaded configuration from .env"
fi

# Get port from environment or use default
VISION_PORT="${VISION_PORT:-7070}"
VISION_P2P_PORT="${VISION_P2P_PORT:-7072}"

# Create data directory
mkdir -p "./vision_data_${VISION_PORT}"
echo "âœ“ Data directory: ./vision_data_${VISION_PORT}"
echo ""

# Check if binary exists and is executable
if [ ! -f "./vision-node" ]; then
    if [ -f "./target/release/vision-node" ]; then
        echo "Using freshly built binary from target/release/"
        cd "$SCRIPT_DIR"
        exec ./target/release/vision-node
    else
        echo "âŒ ERROR: vision-node binary not found!"
        echo ""
        echo "The binary should be in the same directory as this script."
        echo "If you're building from source, run:"
        echo "  cargo build --release"
        echo ""
        exit 1
    fi
fi

# Make binary executable
chmod +x vision-node

# Check if binary is actually executable
if [ ! -x "./vision-node" ]; then
    echo "âŒ ERROR: Cannot execute vision-node binary"
    echo "   This may be a permissions issue or wrong architecture"
    echo ""
    exit 1
fi

# Display startup info
echo "Starting Vision Node..."
echo "Web Interface: http://localhost:${VISION_PORT}"
echo "P2P Listener: 0.0.0.0:${VISION_P2P_PORT}"
echo ""
echo "Press Ctrl+C to stop the node"
echo ""
echo "â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”â”"
echo ""

# Run the node
exec ./vision-node
