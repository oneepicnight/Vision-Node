#!/bin/bash

# ═══════════════════════════════════════════════════════════════════════════
# VISION NODE v1.0.0 - MAINNET INSTALLER
# Professional Installation Script for Linux Systems
# ═══════════════════════════════════════════════════════════════════════════

set -e  # Exit on error

# Color codes for beautiful output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
NC='\033[0m' # No Color

# Banner
clear
echo -e "${MAGENTA}"
cat << "EOF"
╔═══════════════════════════════════════════════════════════════════╗
║                                                                   ║
║   ██╗   ██╗██╗███████╗██╗ ██████╗ ███╗   ██╗                    ║
║   ██║   ██║██║██╔════╝██║██╔═══██╗████╗  ██║                    ║
║   ██║   ██║██║███████╗██║██║   ██║██╔██╗ ██║                    ║
║   ╚██╗ ██╔╝██║╚════██║██║██║   ██║██║╚██╗██║                    ║
║    ╚████╔╝ ██║███████║██║╚██████╔╝██║ ╚████║                    ║
║     ╚═══╝  ╚═╝╚══════╝╚═╝ ╚═════╝ ╚═╝  ╚═══╝                    ║
║                                                                   ║
║              MAINNET INSTALLATION WIZARD v1.0.0                  ║
║                                                                   ║
╚═══════════════════════════════════════════════════════════════════╝
EOF
echo -e "${NC}"

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${WHITE}Welcome to Vision Node Mainnet Installer${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
sleep 1

# ═══════════════════════════════════════════════════════════════════════════
# SECTION 1: SYSTEM CHECKS
# ═══════════════════════════════════════════════════════════════════════════

echo -e "${BLUE}[1/6] System Prerequisites Check${NC}"
echo ""

# Check if running as root (not recommended)
if [ "$EUID" -eq 0 ]; then 
    echo -e "${YELLOW}⚠️  WARNING: Running as root${NC}"
    echo -e "   For security, consider running as a non-root user"
    echo ""
    read -p "Continue anyway? (y/n): " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
else
    echo -e "${GREEN}✅ Running as non-root user${NC}"
fi

# Detect Linux distribution
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$NAME
    VER=$VERSION_ID
    echo -e "${GREEN}✅ Detected: $OS $VER${NC}"
else
    echo -e "${YELLOW}⚠️  Could not detect OS version${NC}"
    OS="Unknown"
fi

# Check system architecture
ARCH=$(uname -m)
if [ "$ARCH" = "x86_64" ]; then
    echo -e "${GREEN}✅ Architecture: x86_64 (AMD64)${NC}"
elif [ "$ARCH" = "aarch64" ]; then
    echo -e "${GREEN}✅ Architecture: ARM64${NC}"
else
    echo -e "${RED}❌ Unsupported architecture: $ARCH${NC}"
    echo -e "   Vision Node requires x86_64 or ARM64"
    exit 1
fi

# Check available disk space (need at least 10GB)
AVAILABLE_SPACE=$(df -BG . | tail -1 | awk '{print $4}' | sed 's/G//')
if [ "$AVAILABLE_SPACE" -lt 10 ]; then
    echo -e "${YELLOW}⚠️  Low disk space: ${AVAILABLE_SPACE}GB available${NC}"
    echo -e "   Recommended: 50GB+ for blockchain data"
else
    echo -e "${GREEN}✅ Disk space: ${AVAILABLE_SPACE}GB available${NC}"
fi

# Check RAM (recommend 4GB+)
TOTAL_RAM=$(free -g | awk '/^Mem:/{print $2}')
if [ "$TOTAL_RAM" -lt 4 ]; then
    echo -e "${YELLOW}⚠️  Low RAM: ${TOTAL_RAM}GB${NC}"
    echo -e "   Recommended: 4GB+ for optimal performance"
else
    echo -e "${GREEN}✅ RAM: ${TOTAL_RAM}GB${NC}"
fi

echo ""

# ═══════════════════════════════════════════════════════════════════════════
# SECTION 2: DEPENDENCY CHECK & INSTALLATION
# ═══════════════════════════════════════════════════════════════════════════

echo -e "${BLUE}[2/6] Dependency Check${NC}"
echo ""

MISSING_DEPS=0
INSTALL_CMD=""

# Determine package manager and install command
if command -v apt-get &> /dev/null; then
    PKG_MANAGER="apt-get"
    UPDATE_CMD="sudo apt-get update -qq"
    INSTALL_CMD="sudo apt-get install -y"
elif command -v yum &> /dev/null; then
    PKG_MANAGER="yum"
    UPDATE_CMD="sudo yum check-update"
    INSTALL_CMD="sudo yum install -y"
elif command -v dnf &> /dev/null; then
    PKG_MANAGER="dnf"
    UPDATE_CMD="sudo dnf check-update"
    INSTALL_CMD="sudo dnf install -y"
elif command -v apk &> /dev/null; then
    PKG_MANAGER="apk"
    UPDATE_CMD="sudo apk update"
    INSTALL_CMD="sudo apk add --no-cache"
else
    echo -e "${YELLOW}⚠️  Could not detect package manager${NC}"
    PKG_MANAGER="manual"
fi

# Check for OpenSSL
if ! ldconfig -p 2>/dev/null | grep -q libssl.so; then
    echo -e "${YELLOW}⚠️  Missing: OpenSSL library${NC}"
    MISSING_DEPS=1
else
    echo -e "${GREEN}✅ OpenSSL library found${NC}"
fi

# Check for libcrypto
if ! ldconfig -p 2>/dev/null | grep -q libcrypto.so; then
    echo -e "${YELLOW}⚠️  Missing: libcrypto library${NC}"
    MISSING_DEPS=1
else
    echo -e "${GREEN}✅ libcrypto library found${NC}"
fi

# Check for curl
if ! command -v curl &> /dev/null; then
    echo -e "${YELLOW}⚠️  Missing: curl${NC}"
    MISSING_DEPS=1
else
    echo -e "${GREEN}✅ curl found${NC}"
fi

# Offer to install missing dependencies
if [ $MISSING_DEPS -eq 1 ] && [ "$PKG_MANAGER" != "manual" ]; then
    echo ""
    echo -e "${YELLOW}Some dependencies are missing.${NC}"
    read -p "Install them automatically? (y/n): " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${CYAN}Installing dependencies...${NC}"
        
        if [ "$PKG_MANAGER" = "apt-get" ]; then
            $UPDATE_CMD
            $INSTALL_CMD libssl-dev curl
        elif [ "$PKG_MANAGER" = "yum" ] || [ "$PKG_MANAGER" = "dnf" ]; then
            $INSTALL_CMD openssl-devel curl
        elif [ "$PKG_MANAGER" = "apk" ]; then
            $UPDATE_CMD
            $INSTALL_CMD openssl-dev curl
        fi
        
        echo -e "${GREEN}✅ Dependencies installed${NC}"
    else
        echo -e "${RED}Cannot proceed without dependencies${NC}"
        exit 1
    fi
fi

echo ""

# ═══════════════════════════════════════════════════════════════════════════
# SECTION 3: INSTALLATION DIRECTORY SETUP
# ═══════════════════════════════════════════════════════════════════════════

echo -e "${BLUE}[3/6] Installation Directory Setup${NC}"
echo ""

# Default installation directory
DEFAULT_INSTALL_DIR="$HOME/vision-node"
echo -e "Default installation directory: ${CYAN}$DEFAULT_INSTALL_DIR${NC}"
read -p "Use default? (y/n): " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    read -p "Enter custom path: " INSTALL_DIR
    INSTALL_DIR="${INSTALL_DIR/#\~/$HOME}"  # Expand ~
else
    INSTALL_DIR="$DEFAULT_INSTALL_DIR"
fi

# Create installation directory
if [ -d "$INSTALL_DIR" ]; then
    echo -e "${YELLOW}⚠️  Directory exists: $INSTALL_DIR${NC}"
    read -p "Overwrite existing installation? (y/n): " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${RED}Installation cancelled${NC}"
        exit 1
    fi
    rm -rf "$INSTALL_DIR"
fi

mkdir -p "$INSTALL_DIR"
echo -e "${GREEN}✅ Created: $INSTALL_DIR${NC}"

# Create data directory
mkdir -p "$INSTALL_DIR/vision_data_7070"
echo -e "${GREEN}✅ Created: $INSTALL_DIR/vision_data_7070${NC}"

# Create logs directory
mkdir -p "$INSTALL_DIR/logs"
echo -e "${GREEN}✅ Created: $INSTALL_DIR/logs${NC}"

echo ""

# ═══════════════════════════════════════════════════════════════════════════
# SECTION 4: BUILD FROM SOURCE (IF NEEDED)
# ═══════════════════════════════════════════════════════════════════════════

echo -e "${BLUE}[4/7] Binary Detection & Build${NC}"
echo ""

# Get the script's directory (where the installation files are)
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Check if this is a source distribution or prebuilt binary
BINARY_PATH=""
IS_SOURCE_INSTALL=false

if [ -f "$SCRIPT_DIR/vision-node" ]; then
    # Prebuilt binary found
    BINARY_PATH="$SCRIPT_DIR/vision-node"
    echo -e "${GREEN}✅ Prebuilt binary detected${NC}"
elif [ -f "$SCRIPT_DIR/target/release/vision-node" ]; then
    # Already built from source
    BINARY_PATH="$SCRIPT_DIR/target/release/vision-node"
    echo -e "${GREEN}✅ Built binary detected${NC}"
else
    # Source distribution - need to build
    IS_SOURCE_INSTALL=true
    echo -e "${CYAN}📦 Source distribution detected${NC}"
    echo ""
    
    # Check for Cargo
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}❌ Rust toolchain not found${NC}"
        echo ""
        echo "This is a source distribution and requires Rust to build."
        echo ""
        echo "Install Rust:"
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo ""
        echo "Then run this installer again."
        exit 1
    fi
    
    RUST_VERSION=$(rustc --version 2>/dev/null || echo "unknown")
    echo -e "${GREEN}✅ Rust toolchain found: $RUST_VERSION${NC}"
    echo ""
    
    # Check for required build dependencies on Linux
    echo -e "${CYAN}Checking build dependencies...${NC}"
    MISSING_BUILD_DEPS=0
    
    # Check for essential build tools
    if ! command -v gcc &> /dev/null && ! command -v clang &> /dev/null; then
        echo -e "${RED}❌ Missing: C compiler (gcc or clang)${NC}"
        MISSING_BUILD_DEPS=1
    else
        echo -e "${GREEN}✅ C compiler found${NC}"
    fi
    
    if ! command -v pkg-config &> /dev/null; then
        echo -e "${YELLOW}⚠️  Missing: pkg-config (recommended)${NC}"
    else
        echo -e "${GREEN}✅ pkg-config found${NC}"
    fi
    
    # Check for OpenSSL development files
    if ! pkg-config --exists openssl 2>/dev/null && ! [ -f /usr/include/openssl/ssl.h ]; then
        echo -e "${RED}❌ Missing: OpenSSL development headers${NC}"
        MISSING_BUILD_DEPS=1
    else
        echo -e "${GREEN}✅ OpenSSL development headers found${NC}"
    fi
    
    if [ $MISSING_BUILD_DEPS -eq 1 ]; then
        echo ""
        echo -e "${RED}Missing required build dependencies${NC}"
        echo ""
        read -p "Install them automatically? (requires sudo) (y/n): " -n 1 -r
        echo ""
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo -e "${CYAN}Installing build dependencies...${NC}"
            
            if [ "$PKG_MANAGER" = "apt-get" ]; then
                sudo apt-get update
                sudo apt-get install -y build-essential pkg-config libssl-dev clang lld
            elif [ "$PKG_MANAGER" = "yum" ]; then
                sudo yum groupinstall -y 'Development Tools'
                sudo yum install -y pkg-config openssl-devel clang lld
            elif [ "$PKG_MANAGER" = "dnf" ]; then
                sudo dnf groupinstall -y 'Development Tools'
                sudo dnf install -y pkg-config openssl-devel clang lld
            else
                echo -e "${YELLOW}Manual installation required:${NC}"
                echo "  Ubuntu/Debian: sudo apt-get install build-essential pkg-config libssl-dev clang lld"
                echo "  CentOS/RHEL: sudo yum groupinstall 'Development Tools' && sudo yum install pkg-config openssl-devel clang lld"
                exit 1
            fi
            
            echo -e "${GREEN}✅ Build dependencies installed${NC}"
        else
            echo -e "${RED}Cannot build without dependencies${NC}"
            exit 1
        fi
    fi
    
    # Detect Cargo.toml location (root or subdirectory)
    BUILD_DIR="$SCRIPT_DIR"
    if [ -f "$SCRIPT_DIR/Cargo.toml" ]; then
        echo -e "${GREEN}✅ Found Cargo.toml in root directory${NC}"
        BUILD_DIR="$SCRIPT_DIR"
    elif [ -f "$SCRIPT_DIR/vision-node/Cargo.toml" ]; then
        echo -e "${GREEN}✅ Found Cargo.toml in vision-node/ subdirectory${NC}"
        BUILD_DIR="$SCRIPT_DIR/vision-node"
    elif [ -f "$SCRIPT_DIR/src/Cargo.toml" ]; then
        echo -e "${GREEN}✅ Found Cargo.toml in src/ subdirectory${NC}"
        BUILD_DIR="$SCRIPT_DIR/src"
    else
        echo -e "${RED}❌ Cargo.toml not found${NC}"
        echo "This doesn't appear to be a valid source package."
        echo ""
        echo "Expected Cargo.toml in one of:"
        echo "  $SCRIPT_DIR/Cargo.toml"
        echo "  $SCRIPT_DIR/vision-node/Cargo.toml"
        echo "  $SCRIPT_DIR/src/Cargo.toml"
        exit 1
    fi
    
    echo -e "${YELLOW}Building Vision Node from source...${NC}"
    echo -e "${CYAN}Build directory: $BUILD_DIR${NC}"
    echo -e "${CYAN}This may take 10-20 minutes depending on your system.${NC}"
    echo ""
    
    # Build with release optimizations
    cd "$BUILD_DIR"
    if cargo build --release 2>&1 | tee "$SCRIPT_DIR/build.log"; then
        echo ""
        echo -e "${GREEN}✅ Build completed successfully!${NC}"
        BINARY_PATH="$BUILD_DIR/target/release/vision-node"
    else
        echo ""
        echo -e "${RED}❌ Build failed${NC}"
        echo ""
        echo "Build log saved to: $SCRIPT_DIR/build.log"
        echo ""
        echo "Common issues:"
        echo "  • Missing system dependencies (OpenSSL, build-essential)"
        echo "  • Insufficient disk space"
        echo "  • Outdated Rust version (update with: rustup update)"
        exit 1
    fi
fi

# Final binary validation
if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}❌ Binary not found after build: $BINARY_PATH${NC}"
    exit 1
fi

# Make binary executable
chmod +x "$BINARY_PATH"

# Get binary size for confirmation
BINARY_SIZE=$(du -h "$BINARY_PATH" | cut -f1)
echo -e "${GREEN}✅ Binary ready: $BINARY_SIZE${NC}"
echo ""

# ═══════════════════════════════════════════════════════════════════════════
# SECTION 5: FILE INSTALLATION
# ═══════════════════════════════════════════════════════════════════════════

echo -e "${BLUE}[5/7] Installing Vision Node Files${NC}"
echo ""

# Copy binary to installation directory
cp "$BINARY_PATH" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/vision-node"
echo -e "${GREEN}✅ Installed: vision-node binary${NC}"

# Copy configuration files
for file in keys.json.example miner.json p2p.json seed_peers.json; do
    if [ -f "$SCRIPT_DIR/$file" ]; then
        cp "$SCRIPT_DIR/$file" "$INSTALL_DIR/"
        echo -e "${GREEN}✅ Installed: $file${NC}"
    fi
done

# Copy config directory (critical for token_accounts.toml)
if [ -d "$SCRIPT_DIR/config" ]; then
    mkdir -p "$INSTALL_DIR/config"
    cp -r "$SCRIPT_DIR/config/"* "$INSTALL_DIR/config/"
    echo -e "${GREEN}✅ Installed: config/ directory${NC}"
    
    # Verify critical config file
    if [ -f "$INSTALL_DIR/config/token_accounts.toml" ]; then
        echo -e "${GREEN}  ✓ token_accounts.toml (required)${NC}"
    else
        echo -e "${YELLOW}  ⚠️  token_accounts.toml missing (node may panic)${NC}"
    fi
fi

# Copy wallet files if they exist
if [ -d "$SCRIPT_DIR/wallet" ]; then
    cp -r "$SCRIPT_DIR/wallet" "$INSTALL_DIR/"
    echo -e "${GREEN}✅ Installed: wallet interface${NC}"
fi

# Copy public files if they exist
if [ -d "$SCRIPT_DIR/public" ]; then
    cp -r "$SCRIPT_DIR/public" "$INSTALL_DIR/"
    echo -e "${GREEN}✅ Installed: public assets${NC}"
fi

# Copy documentation
for file in README.md RELEASE_NOTES.txt NOTICE; do
    if [ -f "$SCRIPT_DIR/$file" ]; then
        cp "$SCRIPT_DIR/$file" "$INSTALL_DIR/"
        echo -e "${GREEN}✅ Installed: $file${NC}"
    fi
done

echo ""

# ═══════════════════════════════════════════════════════════════════════════
# SECTION 6: STARTUP SCRIPT CREATION
# ═══════════════════════════════════════════════════════════════════════════

echo -e "${BLUE}[6/7] Creating Startup Scripts${NC}"
echo ""

# Create start script
cat > "$INSTALL_DIR/start-node.sh" << 'EOFSTART'
#!/bin/bash
# Vision Node Startup Script

cd "$(dirname "$0")"

echo "Starting Vision Node..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Export default environment variables
export VISION_PORT=7070
export VISION_DATA_DIR="./vision_data_7070"
export RUST_LOG=info
export RUST_BACKTRACE=1

# Start the node
./vision-node 2>&1 | tee -a logs/node-$(date +%Y%m%d).log
EOFSTART

chmod +x "$INSTALL_DIR/start-node.sh"
echo -e "${GREEN}✅ Created: start-node.sh${NC}"

# Create systemd service file (optional)
cat > "$INSTALL_DIR/vision-node.service" << EOFSERVICE
[Unit]
Description=Vision Node Mainnet
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$INSTALL_DIR
Environment="VISION_PORT=7070"
Environment="VISION_DATA_DIR=./vision_data_7070"
Environment="RUST_LOG=info"
ExecStart=$INSTALL_DIR/vision-node
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOFSERVICE

echo -e "${GREEN}✅ Created: vision-node.service (systemd)${NC}"

# Create stop script
cat > "$INSTALL_DIR/stop-node.sh" << 'EOFSTOP'
#!/bin/bash
# Vision Node Stop Script

echo "Stopping Vision Node..."
pkill -f vision-node

# Wait for graceful shutdown
sleep 3

# Check if still running
if pgrep -f vision-node > /dev/null; then
    echo "Force stopping..."
    pkill -9 -f vision-node
fi

echo "Vision Node stopped"
EOFSTOP

chmod +x "$INSTALL_DIR/stop-node.sh"
echo -e "${GREEN}✅ Created: stop-node.sh${NC}"

# Create status check script
cat > "$INSTALL_DIR/check-status.sh" << 'EOFSTATUS'
#!/bin/bash
# Vision Node Status Check

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "VISION NODE STATUS"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check if process is running
if pgrep -f vision-node > /dev/null; then
    echo "✅ Node is RUNNING"
    echo ""
    
    # Get process details
    ps aux | grep vision-node | grep -v grep | awk '{printf "   PID: %s\n   Memory: %s\n   CPU: %s%%\n", $2, $6/1024"MB", $3}'
    echo ""
    
    # Query health endpoint
    echo "API Health Check:"
    curl -s http://localhost:7070/health | jq '.' 2>/dev/null || curl -s http://localhost:7070/health
else
    echo "❌ Node is NOT RUNNING"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
EOFSTATUS

chmod +x "$INSTALL_DIR/check-status.sh"
echo -e "${GREEN}✅ Created: check-status.sh${NC}"

echo ""

# ═══════════════════════════════════════════════════════════════════════════
# SECTION 7: SYSTEMD SERVICE INSTALLATION (OPTIONAL)
# ═══════════════════════════════════════════════════════════════════════════

echo -e "${BLUE}[7/7] System Service Setup (Optional)${NC}"
echo ""

echo -e "Install as system service? (Requires sudo)"
echo -e "  ${GREEN}• Auto-start on boot${NC}"
echo -e "  ${GREEN}• Automatic restart on crash${NC}"
echo -e "  ${GREEN}• System logging integration${NC}"
echo ""
read -p "Install systemd service? (y/n): " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    if command -v systemctl &> /dev/null; then
        sudo cp "$INSTALL_DIR/vision-node.service" /etc/systemd/system/
        sudo systemctl daemon-reload
        sudo systemctl enable vision-node.service
        echo -e "${GREEN}✅ Systemd service installed and enabled${NC}"
        echo ""
        echo -e "Service commands:"
        echo -e "  ${CYAN}Start:${NC}   sudo systemctl start vision-node"
        echo -e "  ${CYAN}Stop:${NC}    sudo systemctl stop vision-node"
        echo -e "  ${CYAN}Status:${NC}  sudo systemctl status vision-node"
        echo -e "  ${CYAN}Logs:${NC}    sudo journalctl -u vision-node -f"
    else
        echo -e "${YELLOW}⚠️  systemd not found - skipping service installation${NC}"
    fi
fi

echo ""

# ═══════════════════════════════════════════════════════════════════════════
# INSTALLATION COMPLETE
# ═══════════════════════════════════════════════════════════════════════════

clear
echo -e "${GREEN}"
cat << "EOF"
╔═══════════════════════════════════════════════════════════════════╗
║                                                                   ║
║              ✅ INSTALLATION COMPLETE! ✅                         ║
║                                                                   ║
╚═══════════════════════════════════════════════════════════════════╝
EOF
echo -e "${NC}"

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${WHITE}Installation Summary${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "   ${WHITE}Installation Directory:${NC} ${CYAN}$INSTALL_DIR${NC}"
echo -e "   ${WHITE}Data Directory:${NC}         ${CYAN}$INSTALL_DIR/vision_data_7070${NC}"
echo -e "   ${WHITE}Logs Directory:${NC}         ${CYAN}$INSTALL_DIR/logs${NC}"
echo ""

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${WHITE}Quick Start Guide${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${YELLOW}1. Navigate to installation directory:${NC}"
echo -e "   ${CYAN}cd $INSTALL_DIR${NC}"
echo ""
echo -e "${YELLOW}2. Start the node:${NC}"
echo -e "   ${CYAN}./start-node.sh${NC}"
echo ""
echo -e "${YELLOW}3. Access web interface:${NC}"
echo -e "   ${CYAN}http://localhost:7070${NC}"
echo ""
echo -e "${YELLOW}4. Check node status:${NC}"
echo -e "   ${CYAN}./check-status.sh${NC}"
echo ""
echo -e "${YELLOW}5. Stop the node:${NC}"
echo -e "   ${CYAN}./stop-node.sh${NC}"
echo ""

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${WHITE}Firewall Configuration${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${YELLOW}Allow incoming connections (required for peer connectivity):${NC}"
echo ""

if command -v ufw &> /dev/null; then
    echo -e "${WHITE}UFW:${NC}"
    echo -e "   ${CYAN}sudo ufw allow 7070/tcp${NC}"
    echo -e "   ${CYAN}sudo ufw allow 7072/tcp${NC}"
    echo ""
fi

if command -v firewall-cmd &> /dev/null; then
    echo -e "${WHITE}Firewalld:${NC}"
    echo -e "   ${CYAN}sudo firewall-cmd --permanent --add-port=7070/tcp${NC}"
    echo -e "   ${CYAN}sudo firewall-cmd --permanent --add-port=7072/tcp${NC}"
    echo -e "   ${CYAN}sudo firewall-cmd --reload${NC}"
    echo ""
fi

echo -e "${WHITE}iptables:${NC}"
echo -e "   ${CYAN}sudo iptables -A INPUT -p tcp --dport 7070 -j ACCEPT${NC}"
echo -e "   ${CYAN}sudo iptables -A INPUT -p tcp --dport 7072 -j ACCEPT${NC}"
echo ""

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${WHITE}Important Notes${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "   ${YELLOW}•${NC} Node requires ${WHITE}3+ peer connections${NC} to sync and mine"
echo -e "   ${YELLOW}•${NC} Blockchain data will grow over time (plan for 50GB+)"
echo -e "   ${YELLOW}•${NC} First sync may take several hours depending on network"
echo -e "   ${YELLOW}•${NC} Configure wallet in web interface to enable mining"
echo ""

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${WHITE}Need Help?${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "   ${WHITE}Documentation:${NC} $INSTALL_DIR/README.md"
echo -e "   ${WHITE}Release Notes:${NC} $INSTALL_DIR/RELEASE_NOTES.txt"
echo ""

echo -e "${GREEN}Happy mining! 🚀${NC}"
echo ""
