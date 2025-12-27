# VisionNode v3.0.0 WAN Environment (Public Test)
# Optional configuration overrides. The node works out-of-the-box with defaults.

# --- Network Ports (optional) ---
# $env:VISION_PORT = "7070"            # HTTP API/UI port (default 7070)
# $env:VISION_P2P_PORT = "7072"        # P2P mesh port (default 7072)

# --- Public IP Advertisement (recommended if behind NAT) ---
# Set your public IP if auto-detection fails or you're behind NAT/firewall.
# $env:VISION_PUBLIC_IP = "203.0.113.10"   # Your external IPv4
# $env:VISION_PUBLIC_PORT = "7072"         # External P2P port

# --- Anchor Seeds (optional) ---
# HTTP control plane seeds. If unset, uses built-in genesis anchors.
# $env:VISION_ANCHOR_SEEDS = "35.151.236.81,16.163.123.221"

# Access the wallet and dashboard at http://localhost:7070 after starting.
