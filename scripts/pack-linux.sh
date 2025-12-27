#!/usr/bin/env bash
set -euo pipefail

echo "📦 Vision Node Linux Packager"
echo "=============================="

# Read version
if [ ! -f "VERSION" ]; then
    echo "ERROR: VERSION file not found"
    exit 1
fi
ver="$(cat VERSION | tr -d '\r\n')"
echo "Version: $ver"

# Build single-world variant (FULL)
echo ""
echo "🔨 Building FULL variant..."
cargo build --release

# Create dist directory
distDir="dist/VisionNode-$ver-Linux"
echo ""
echo "📁 Creating package: $distDir"
mkdir -p "$distDir"

# Copy binary
echo "  ✓ Copying binary..."
cp target/release/vision-node "$distDir/"
chmod +x "$distDir/vision-node"

# Copy VERSION
echo "  ✓ Copying VERSION..."
cp VERSION "$distDir/"

# Copy docs
echo "  ✓ Copying documentation..."
if [ -d "docs" ]; then
    cp -r docs "$distDir/docs"
fi

# Copy config
echo "  ✓ Copying config..."
if [ -d "config" ]; then
    cp -r config "$distDir/config"
fi

# Create run script
echo "  ✓ Creating run-linux.sh..."
cat > "$distDir/run-linux.sh" <<'RUNSCRIPT'
#!/usr/bin/env bash
echo "Vision Node - Linux Launcher"
echo "============================"
echo ""
echo "Configuration:"
echo "- Port: 7070"
echo "- Data dir: ./data"
echo ""
echo "Set these environment variables before running:"
echo "  export VISION_ADMIN_TOKEN=your-secret-token"
echo "  export VISION_ALLOW_SEED=1  # for testing only"
echo ""

if [ -z "${VISION_ADMIN_TOKEN:-}" ]; then
    echo "WARNING: VISION_ADMIN_TOKEN not set!"
    echo "Set it with: export VISION_ADMIN_TOKEN=your-secret-token"
    echo ""
fi

./vision-node --port 7070 --data ./data
RUNSCRIPT

chmod +x "$distDir/run-linux.sh"

# Create START-VISION-NODE.sh for constellation mode
echo "  ✓ Creating START-VISION-NODE.sh..."
cat > "$distDir/START-VISION-NODE.sh" <<'STARTSCRIPT'
#!/usr/bin/env bash
#
# Vision Node - Constellation Mode Launcher
# =========================================

echo "🌟 Starting Vision Node (Constellation mode)..."
echo ""

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Check if binary exists
if [ ! -f "./vision-node" ]; then
    if [ -f "./target/release/vision-node" ]; then
        echo "Using freshly built binary from target/release/"
        cd "$SCRIPT_DIR"
        exec ./target/release/vision-node --role constellation
    else
        echo "❌ ERROR: vision-node binary not found!"
        echo ""
        echo "Please build the node first:"
        echo "  cargo build --release"
        echo ""
        echo "Or if you have the binary, place it in this directory."
        exit 1
    fi
fi

# Run the node in constellation mode
exec ./vision-node --role constellation
STARTSCRIPT

chmod +x "$distDir/START-VISION-NODE.sh"

# Create README
echo "  ✓ Creating README.txt..."
cat > "$distDir/README.txt" <<README
Vision Node - FULL Build (Single World)
==============================

Version: $ver
Build: Linux 64-bit
Features: FULL-only (single build world)

Quick Start:
1. Set environment variables:
   export VISION_ADMIN_TOKEN=your-secret-token
   export VISION_ALLOW_SEED=1

2. Run the node:
   ./run-linux.sh

3. Test it:
   curl http://localhost:7070/status

Documentation:
- See docs/ folder for API reference
- See docs/MVP_ENDPOINTS.md for available routes
- See BUILD_VARIANTS.md for build instructions

Configuration:
- Port: Set via --port flag (default: 7070)
- Data dir: Set via --data flag (default: ./data)
- Admin token: VISION_ADMIN_TOKEN environment variable
- CORS origins: VISION_CORS_ORIGINS (comma-separated)

MVP Endpoints (stable surface) include:
- Wallet operations (balance, transfer, receipts)
- Transaction submission & fee estimation
- Block & chain queries
- Staking & tokenomics
- P2P networking & sync
- Admin operations
- Prometheus metrics
- Snapshot management

Build:
    cargo build --release

Support: https://github.com/yourusername/vision-node
README

# Create tarball
echo ""
echo "📦 Creating tarball..."
tarball="dist/VisionNode-$ver-Linux.tar.gz"
tar -C dist -czf "$tarball" "VisionNode-$ver-Linux"

# Calculate checksum
echo ""
echo "🔐 Calculating SHA256 checksum..."
if command -v sha256sum &> /dev/null; then
    hash=$(sha256sum "$tarball" | cut -d' ' -f1)
else
    hash=$(shasum -a 256 "$tarball" | cut -d' ' -f1)
fi
echo "  SHA256: $hash"

# Save checksum
echo "$hash" > "$tarball.sha256"

# Summary
echo ""
echo "✅ Package created successfully!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Package: $tarball"
echo "  Size: $(du -h "$tarball" | cut -f1)"
echo "  SHA256: $hash"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
