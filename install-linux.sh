#!/bin/bash

# Vision Node v2.0.0 Installation Script (Constellation)

echo ""
echo "========================================"
echo "  VISION NODE v2.0.0 INSTALLER"
echo "  MODE: CONSTELLATION (SWARM ONLY)"
echo "========================================"
echo ""

# Make scripts and binary executable
chmod +x vision-node 2>/dev/null || true
chmod +x START-PUBLIC-NODE.sh 2>/dev/null || true
chmod +x START-VISION-NODE.sh 2>/dev/null || true

echo "✅ Made scripts executable"

# Create data directory
VISION_PORT="${VISION_PORT:-7070}"
mkdir -p "./vision_data_${VISION_PORT}"
echo "✅ Created data directory: ./vision_data_${VISION_PORT}"

# Check dependencies
echo ""
echo "Checking system dependencies..."

# Check if running as root
if [ "$EUID" -eq 0 ]; then 
    echo "⚠️  Running as root - this is not recommended"
    echo "   Consider creating a dedicated user for the node"
fi

# Check for required libraries
MISSING_DEPS=0

if ! ldconfig -p | grep -q libssl.so; then
    echo "❌ Missing: libssl (OpenSSL)"
    MISSING_DEPS=1
fi

if ! ldconfig -p | grep -q libcrypto.so; then
    echo "❌ Missing: libcrypto (OpenSSL)"
    MISSING_DEPS=1
fi

if [ $MISSING_DEPS -eq 1 ]; then
    echo ""
    echo "To install missing dependencies:"
    echo ""
    echo "Ubuntu/Debian:"
    echo "  sudo apt-get update"
    echo "  sudo apt-get install -y libssl-dev"
    echo ""
    echo "CentOS/RHEL:"
    echo "  sudo yum install -y openssl-devel"
    echo ""
    echo "Alpine:"
    echo "  apk add --no-cache openssl-dev"
    echo ""
else
    echo "✅ All required libraries found"
fi

echo ""
echo "Installation complete!"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "QUICK START:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "1. Start the node:"
echo "   ./START-PUBLIC-NODE.sh"
echo ""
echo "2. Access web interface:"
echo "   http://localhost:7070"
echo ""
echo "3. Check node status:"
echo "   curl http://localhost:7070/health"
echo "   curl http://localhost:7070/constellation/status"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "CONFIGURATION:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Edit .env file to customize:"
echo "  - VISION_PORT (HTTP API port, default: 7070)"
echo "  - VISION_P2P_PORT (P2P port, default: 7072)"
echo "  - VISION_UPNP_ENABLED (auto port-forward, default: true)"
echo "  - VISION_IPV4_ONLY (IPv4-only mode, default: true)"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "FIREWALL SETUP:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Allow incoming connections (required for peers):"
echo ""
echo "UFW (Ubuntu/Debian):"
echo "  sudo ufw allow 7070/tcp"
echo "  sudo ufw allow 7072/tcp"
echo "  sudo ufw reload"
echo ""
echo "Firewalld (CentOS/RHEL):"
echo "  sudo firewall-cmd --permanent --add-port=7070/tcp"
echo "  sudo firewall-cmd --permanent --add-port=7072/tcp"
echo "  sudo firewall-cmd --reload"
echo ""
echo "iptables:"
echo "  sudo iptables -A INPUT -p tcp --dport 7070 -j ACCEPT"
echo "  sudo iptables -A INPUT -p tcp --dport 7072 -j ACCEPT"
echo "  sudo iptables-save > /etc/iptables/rules.v4"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
