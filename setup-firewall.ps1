# Setup Windows Firewall for Vision Node
# Run this script as Administrator

Write-Host "Setting up Windows Firewall rules for Vision Node..." -ForegroundColor Cyan
Write-Host ""

# Check if running as administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "ERROR: This script must be run as Administrator!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Right-click PowerShell and select 'Run as Administrator', then run this script again." -ForegroundColor Yellow
    pause
    exit 1
}

# Remove existing rules if they exist
Write-Host "Removing any existing Vision Node firewall rules..."
netsh advfirewall firewall delete rule name="Vision Node HTTP" 2>$null
netsh advfirewall firewall delete rule name="Vision Node P2P" 2>$null

# Add HTTP API rule (port 7070)
Write-Host "Adding rule for HTTP API (port 7070)..."
netsh advfirewall firewall add rule name="Vision Node HTTP" dir=in action=allow protocol=TCP localport=7070

# Add P2P rule (port 7071)
Write-Host "Adding rule for P2P (port 7071)..."
netsh advfirewall firewall add rule name="Vision Node P2P" dir=in action=allow protocol=TCP localport=7071

Write-Host ""
Write-Host "Firewall rules added successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "Ports opened:"
Write-Host "  - 7070: HTTP API"
Write-Host "  - 7071: TCP P2P (peer connections)"
Write-Host ""
Write-Host "Next steps:"
Write-Host "1. Configure port forwarding on your router:"
Write-Host "   - Forward external port 7071 -> 192.168.1.123:7071 (P2P)"
Write-Host "   - Forward external port 7070 -> 192.168.1.123:7070 (HTTP, optional)"
Write-Host ""
Write-Host "2. Start your public node"
Write-Host ""
Write-Host "3. Miners connect to: 12.74.244.112:7071"
Write-Host ""
pause
