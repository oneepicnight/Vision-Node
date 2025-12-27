#!/bin/bash

# Vision Node v2.0.0 Startup Script

echo ""
echo "========================================"
echo "  STARTING VISION NODE v2.0.0"
echo "  MODE: CONSTELLATION"
echo "========================================"
echo ""

# Check if binary exists
if [ ! -f "./target/release/vision-node" ]; then
    echo " Binary not found: ./target/release/vision-node"
    echo ""
    echo "Please run ./install.sh first to build the node."
    exit 1
fi

# Check if .env exists
if [ ! -f ".env" ]; then
    echo "  .env file not found"
    if [ -f ".env.example" ]; then
        echo "Creating .env from .env.example..."
        cp .env.example .env
        echo ""
        echo "  Please edit .env and set your VISION_WALLET_ADDRESS before starting"
        exit 1
    else
        echo " No .env.example found"
        exit 1
    fi
fi

# Load environment variables
if [ -f ".env" ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

# Check for wallet address
if [ -z "$VISION_WALLET_ADDRESS" ] || [ "$VISION_WALLET_ADDRESS" = "your_wallet_address_here" ]; then
    echo " VISION_WALLET_ADDRESS not set in .env"
    echo ""
    echo "Please edit .env and set your wallet address:"
    echo "  VISION_WALLET_ADDRESS=your_actual_wallet_address"
    echo ""
    exit 1
fi

# Show configuration
echo "Configuration:"
echo "  Wallet:   $VISION_WALLET_ADDRESS"
echo "  HTTP Port: ${VISION_PORT:-7070}"
echo "  P2P Port:  ${VISION_P2P_PORT:-7072}"
echo "  UPnP:      ${VISION_UPNP_ENABLED:-false}"
echo ""

echo "Starting node..."
echo ""
echo "Access points:"
echo "  Wallet:    http://localhost:${VISION_PORT:-7070}/app"
echo "  Panel:     http://localhost:${VISION_PORT:-7070}/panel.html"
echo "  Dashboard: http://localhost:${VISION_PORT:-7070}/dashboard.html"
echo "  API:       http://localhost:${VISION_PORT:-7070}/constellation/status"
echo ""
echo "Press Ctrl+C to stop the node"
echo ""
echo "========================================"
echo ""

# Run the node
./target/release/vision-node