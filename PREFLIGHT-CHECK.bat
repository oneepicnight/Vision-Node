@echo off
title Pre-Flight Checklist - Public Node

echo.
echo ================================================
echo   Public Node Pre-Flight Checklist
echo ================================================
echo.

echo [1] Checking if vision-node.exe exists...
if exist "%~dp0vision-node.exe" (
    echo     [OK] vision-node.exe found
) else (
    echo     [FAIL] vision-node.exe not found!
    goto :error
)

echo.
echo [2] Network Configuration:
echo     Your Public IP: 12.74.244.112
echo     Your Local IP:  192.168.1.123
echo     P2P Port:       7071
echo.

echo [3] Firewall Configuration:
echo     Run this as Administrator if not done:
echo     setup-firewall.ps1
echo.
powershell -Command "& {$rules = netsh advfirewall firewall show rule name=all | Select-String 'Vision Node'; if ($rules) { Write-Host '     [OK] Firewall rules found' -ForegroundColor Green } else { Write-Host '     [WARNING] No firewall rules found - Run setup-firewall.ps1 as Admin' -ForegroundColor Yellow }}"

echo.
echo [4] Router Port Forwarding:
echo     You need to configure this manually on your router:
echo     - External Port: 7071
echo     - Internal IP:   192.168.1.123  
echo     - Internal Port: 7071
echo     - Protocol:      TCP
echo.

echo [5] Test External Access:
echo     After starting the node, test at:
echo     https://www.yougetsignal.com/tools/open-ports/
echo     Enter: 12.74.244.112 and port 7071
echo.

echo ================================================
echo   Checklist Complete
echo ================================================
echo.
echo If firewall and router are configured, you're ready!
echo.
echo Next: Run START-PUBLIC-NODE-INTERNET.bat
echo.
pause
exit /b 0

:error
echo.
echo Setup incomplete. Please fix the errors above.
pause
exit /b 1
