@echo off
echo Vision Wallet Diagnostics
echo.
PowerShell -NoProfile -ExecutionPolicy Bypass -File "%~dp0diagnose-wallet.ps1"
pause
