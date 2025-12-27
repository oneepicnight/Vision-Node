# Vision Blockchain Bootstrap Node Setup
# Run this script as Administrator to configure your laptop as a bootstrap node

Write-Host "`n╔════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║     VISION BLOCKCHAIN - Bootstrap Node Setup              ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

# Check if running as Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "⚠️  ERROR: This script must be run as Administrator" -ForegroundColor Red
    Write-Host "`nRight-click on PowerShell and select 'Run as Administrator'`n" -ForegroundColor Yellow
    pause
    exit 1
}

Write-Host "✅ Running as Administrator`n" -ForegroundColor Green

# Step 1: Configure Windows Firewall
Write-Host "📋 Step 1: Configuring Windows Firewall..." -ForegroundColor Yellow

try {
    # Check if rule already exists
    $existingRule = Get-NetFirewallRule -DisplayName "Vision Blockchain Node" -ErrorAction SilentlyContinue
    
    if ($existingRule) {
        Write-Host "   ℹ️  Firewall rule already exists, updating..." -ForegroundColor Cyan
        Remove-NetFirewallRule -DisplayName "Vision Blockchain Node" -ErrorAction SilentlyContinue
    }
    
    # Create new rule
    New-NetFirewallRule `
        -DisplayName "Vision Blockchain Node" `
        -Direction Inbound `
        -LocalPort 7070 `
        -Protocol TCP `
        -Action Allow `
        -Profile Any `
        -Enabled True | Out-Null
    
    Write-Host "   ✅ Windows Firewall configured (port 7070 open)`n" -ForegroundColor Green
} catch {
    Write-Host "   ❌ Failed to configure firewall: $_" -ForegroundColor Red
    Write-Host "   You may need to configure it manually.`n" -ForegroundColor Yellow
}

# Step 2: Display network information
Write-Host "📋 Step 2: Network Information" -ForegroundColor Yellow

$publicIp = "12.74.244.112"
$localIp = (Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.InterfaceAlias -notlike "*Loopback*" } | Select-Object -First 1).IPAddress

Write-Host "   🌐 Public IP:  $publicIp" -ForegroundColor Cyan
Write-Host "   🏠 Local IP:   $localIp" -ForegroundColor Cyan
Write-Host "   🔌 Port:       7070`n" -ForegroundColor Cyan

# Step 3: Router port forwarding instructions
Write-Host "📋 Step 3: Configure Router Port Forwarding" -ForegroundColor Yellow
Write-Host "   You need to manually configure your router:`n" -ForegroundColor White
Write-Host "   1. Open router admin (usually 192.168.1.1)" -ForegroundColor White
Write-Host "   2. Find 'Port Forwarding' or 'Virtual Server'" -ForegroundColor White
Write-Host "   3. Add new rule:" -ForegroundColor White
Write-Host "      • External Port: 7070" -ForegroundColor Cyan
Write-Host "      • Internal Port: 7070" -ForegroundColor Cyan
Write-Host "      • Internal IP:   $localIp" -ForegroundColor Cyan
Write-Host "      • Protocol:      TCP" -ForegroundColor Cyan
Write-Host "   4. Save and apply`n" -ForegroundColor White

# Step 4: Create bootstrap config
Write-Host "📋 Step 4: Creating Bootstrap Configuration..." -ForegroundColor Yellow

$configPath = Join-Path $PSScriptRoot "config.env"
$adminToken = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 32 | ForEach-Object { [char]$_ })

$configContent = @"
VISION_PORT=7070
VISION_ADMIN_TOKEN=$adminToken
VISION_BOOTSTRAP=
VISION_SOLO=false
"@

try {
    $configContent | Out-File -FilePath $configPath -Encoding ASCII -Force
    Write-Host "   ✅ Configuration file created: $configPath`n" -ForegroundColor Green
} catch {
    Write-Host "   ⚠️  Could not create config file: $_`n" -ForegroundColor Yellow
}

# Step 5: Disable sleep mode
Write-Host "📋 Step 5: Configuring Power Settings..." -ForegroundColor Yellow

try {
    # Keep laptop awake when plugged in
    powercfg /change standby-timeout-ac 0 | Out-Null
    powercfg /change monitor-timeout-ac 30 | Out-Null
    Write-Host "   ✅ Sleep mode disabled when plugged in`n" -ForegroundColor Green
} catch {
    Write-Host "   ⚠️  Could not change power settings`n" -ForegroundColor Yellow
}

# Summary
Write-Host "`n╔════════════════════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║                  SETUP COMPLETE!                           ║" -ForegroundColor Green
Write-Host "╚════════════════════════════════════════════════════════════╝`n" -ForegroundColor Green

Write-Host "📌 Your Bootstrap Node Details:" -ForegroundColor Cyan
Write-Host "   • Public URL:  http://$publicIp:7070" -ForegroundColor White
Write-Host "   • Local URL:   http://localhost:7070" -ForegroundColor White
Write-Host "   • Admin Token: $adminToken`n" -ForegroundColor White

Write-Host "🎯 Next Steps:" -ForegroundColor Yellow
Write-Host "   1. Configure router port forwarding (see instructions above)" -ForegroundColor White
Write-Host "   2. Start your node: .\target\release\vision-node.exe" -ForegroundColor White
Write-Host "   3. Test from another network: curl http://$publicIp:7070/api/status" -ForegroundColor White
Write-Host "   4. Share with testers: http://$publicIp:7070`n" -ForegroundColor White

Write-Host "📖 Documentation:" -ForegroundColor Yellow
Write-Host "   • Full guide: BOOTSTRAP_NODE_SETUP.md" -ForegroundColor White
Write-Host "   • Tester guide: TESTER_NETWORK_MODE.md`n" -ForegroundColor White

Write-Host "🔐 Important Notes:" -ForegroundColor Yellow
Write-Host "   ⚠️  Keep your laptop plugged in and awake" -ForegroundColor White
Write-Host "   ⚠️  Your public IP may change (check periodically)" -ForegroundColor White
Write-Host "   ⚠️  Keep admin token secure`n" -ForegroundColor White

Write-Host "✨ Ready to start your bootstrap node!`n" -ForegroundColor Green
pause
