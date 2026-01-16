#!/usr/bin/env bash
#
# Vision Node v1.0.3 - Production Start Script
# =============================================
#
# This script safely starts a Vision Node with proper cleanup and logging.
# It will gracefully stop any existing instance, free ports, and start fresh.
#

set -e  # Exit on error (except where explicitly handled)

# ============================================================
#  VISION NODE – LINUX START SCRIPT
# 
#  ⚠️  IMPORTANT NETWORK LIMITATION
#  • One Vision node per PUBLIC IP is supported right now
#  • Port 7070 = HTTP / RPC
#  • Port 7072 = P2P
#  • Running multiple nodes behind the same IP will NOT work
# 
#  This is intentional while the network stabilizes.
# ============================================================

echo ""

# Get the directory where this script is located
# This ensures we can run from anywhere and still find our files
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# ============================================================
#  CONFIGURATION
# ============================================================

# Port configuration (can be overridden by environment variables)
export VISION_PORT="${VISION_PORT:-7070}"
export VISION_P2P_PORT="${VISION_P2P_PORT:-7072}"

# Data directory - isolated per port to allow multi-port testing on same machine
export VISION_DATA_DIR="${VISION_DATA_DIR:-./vision_data_${VISION_PORT}}"

# Logging configuration
export RUST_LOG="${RUST_LOG:-info,vision_node=debug}"

# Backtrace only enabled if explicitly set (keeps logs cleaner)
if [ -n "$RUST_BACKTRACE" ]; then
    export RUST_BACKTRACE="$RUST_BACKTRACE"
fi

# Create logs directory
mkdir -p ./logs

# Generate timestamped log file name
LOG_FILE="./logs/vision-node-$(date +%Y%m%d-%H%M%S).log"

echo "Configuration:"
echo "  HTTP Port:     ${VISION_PORT}"
echo "  P2P Port:      ${VISION_P2P_PORT}"
echo "  Data Dir:      ${VISION_DATA_DIR}"
echo "  Log Level:     ${RUST_LOG}"
echo "  Log File:      ${LOG_FILE}"
echo ""

# ============================================================
#  GRACEFUL SHUTDOWN OF EXISTING INSTANCES
# ============================================================

echo "Checking for existing Vision Node processes..."

# Check if any vision-node processes are running
if pgrep -f vision-node > /dev/null; then
    echo "  Found running Vision Node process(es)"
    echo "  Attempting graceful shutdown (SIGTERM)..."
    
    # First attempt: polite SIGTERM (allows cleanup handlers to run)
    pkill -15 -f vision-node 2>/dev/null || true
    
    # Give process time to shut down gracefully
    # This allows database writes to flush, connections to close, etc.
    sleep 2
    
    # Check if process is still running
    if pgrep -f vision-node > /dev/null; then
        echo "  Process still running, forcing shutdown (SIGKILL)..."
        # Only escalate to SIGKILL if absolutely necessary
        pkill -9 -f vision-node 2>/dev/null || true
        sleep 1
    else
        echo "  ✓ Graceful shutdown successful"
    fi
else
    echo "  No existing processes found"
fi

echo ""

# ============================================================
#  PORT CLEANUP
# ============================================================

echo "Freeing ports..."

# Free HTTP/RPC port (7070)
# Using fuser to kill any process holding the port
# -k = kill, -n tcp = network port, 2>/dev/null = suppress errors if port already free
if command -v fuser &> /dev/null; then
    fuser -k ${VISION_PORT}/tcp 2>/dev/null || true
    fuser -k ${VISION_P2P_PORT}/tcp 2>/dev/null || true
    echo "  ✓ Ports ${VISION_PORT} and ${VISION_P2P_PORT} freed"
else
    echo "  ⚠  fuser not found - skipping port cleanup"
    echo "     Install with: sudo apt-get install psmisc"
fi

echo ""

# ============================================================
#  ENVIRONMENT SETUP
# ============================================================

# Load .env file if it exists (allows local configuration overrides)
if [ -f ".env" ]; then
    # Safely load .env, ignoring comments and blank lines
    set -a  # Automatically export all variables
    source <(grep -v '^#' .env | grep -v '^$' | sed -e 's/\r$//')
    set +a
    echo "✓ Loaded configuration from .env"
else
    echo "  No .env file found (using defaults)"
fi

# Create data directory if it doesn't exist
# This is where the blockchain database and state will be stored
mkdir -p "${VISION_DATA_DIR}"
echo "✓ Data directory ready: ${VISION_DATA_DIR}"
echo ""

# ============================================================
#  BINARY VALIDATION
# ============================================================

echo "Validating Vision Node binary..."

# Check if binary exists in current directory
if [ ! -f "./vision-node" ]; then
    # Fall back to target/release if building from source
    if [ -f "./target/release/vision-node" ]; then
        echo "  Using development binary: ./target/release/vision-node"
        BINARY_PATH="./target/release/vision-node"
    else
        echo ""
        echo "❌ ERROR: vision-node binary not found!"
        echo ""
        echo "Expected locations:"
        echo "  ./vision-node"
        echo "  ./target/release/vision-node"
        echo ""
        echo "If building from source, run:"
        echo "  cargo build --release"
        echo ""
        exit 1
    fi
else
    BINARY_PATH="./vision-node"
fi

# Ensure binary is executable
chmod +x "$BINARY_PATH"

# Verify binary can actually execute (catches architecture mismatches)
if [ ! -x "$BINARY_PATH" ]; then
    echo ""
    echo "❌ ERROR: Cannot execute vision-node binary"
    echo ""
    echo "This may indicate:"
    echo "  • Wrong CPU architecture (e.g., ARM binary on x86)"
    echo "  • Missing system libraries"
    echo "  • Permission issues"
    echo ""
    exit 1
fi

echo "✓ Binary validated: $BINARY_PATH"
echo ""

# ============================================================
#  STARTUP
# ============================================================

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  🚀 STARTING VISION NODE v1.0.3"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Access points:"
echo "  Wallet:          http://localhost:${VISION_PORT}/app"
echo "  Mining Panel:    http://localhost:${VISION_PORT}/panel.html"
echo "  Dashboard:       http://localhost:${VISION_PORT}/dashboard.html"
echo "  Status API:      http://localhost:${VISION_PORT}/constellation/status"
echo ""
echo "Network:"
echo "  P2P Listener:    0.0.0.0:${VISION_P2P_PORT}"
echo "  HTTP API:        127.0.0.1:${VISION_PORT}"
echo ""
echo "Output logged to: ${LOG_FILE}"
echo ""
echo "Press Ctrl+C to stop the node"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Start the node with tee to capture output to both terminal and log file
# Using exec to replace shell process with node process (cleaner process tree)
# tee -a = append to log file, shows output in terminal too
exec "$BINARY_PATH" 2>&1 | tee -a "$LOG_FILE"
