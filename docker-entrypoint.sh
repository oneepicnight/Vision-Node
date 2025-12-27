#!/bin/bash
# Docker entrypoint script for Vision Node
# Handles graceful shutdown and initialization

set -e

echo "🚀 Vision Node - Production Deployment"
echo "========================================"

# Function to handle shutdown gracefully
shutdown() {
    echo "🛑 Received shutdown signal, stopping Vision Node..."
    if [ -n "$VISION_PID" ]; then
        kill -TERM "$VISION_PID" 2>/dev/null || true
        wait "$VISION_PID" 2>/dev/null || true
    fi
    echo "✅ Vision Node stopped gracefully"
    exit 0
}

# Trap SIGTERM and SIGINT for graceful shutdown
trap shutdown SIGTERM SIGINT

# Display configuration
echo "Configuration:"
echo "  Port: ${VISION_PORT:-7070}"
echo "  Data Dir: ${VISION_DATA_DIR:-/app/data}"
echo "  Log Level: ${RUST_LOG:-info}"
echo "  Build: FULL"
echo ""

# Ensure data directories exist
mkdir -p "${VISION_DATA_DIR:-/app/data}"
mkdir -p "${VISION_LOG_DIR:-/app/logs}"

# Check if this is first run
if [ ! -f "${VISION_DATA_DIR}/.initialized" ]; then
    echo "📦 First run detected - initializing..."
    touch "${VISION_DATA_DIR}/.initialized"
    echo "   Database initialized"
fi

# Display peer configuration if set
if [ -n "$VISION_BOOTNODES" ]; then
    echo "🌐 Bootstrap nodes configured:"
    echo "   $VISION_BOOTNODES"
fi

echo ""
echo "🎯 Starting Vision Node..."
echo "========================================"

# Start the node in background and capture PID
"$@" &
VISION_PID=$!

# Wait for the process
wait "$VISION_PID"
EXIT_CODE=$?

echo "Vision Node exited with code $EXIT_CODE"
exit $EXIT_CODE
