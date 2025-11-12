#!/usr/bin/env pwsh
# Vision Wallet - Quick Start Script
# This script will install dependencies and start the development server

Write-Host "🌟 Vision Wallet - Quick Start Setup" -ForegroundColor Cyan
Write-Host "===================================" -ForegroundColor Cyan
Write-Host ""

# Check if Node.js is installed
try {
    $nodeVersion = node --version
    Write-Host "✓ Node.js found: $nodeVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ Node.js not found. Please install Node.js from https://nodejs.org/" -ForegroundColor Red
    Write-Host "   Minimum required version: Node.js 16+" -ForegroundColor Yellow
    Read-Host "Press Enter to exit"
    exit 1
}

# Check if npm is available
try {
    $npmVersion = npm --version
    Write-Host "✓ npm found: v$npmVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ npm not found. Please reinstall Node.js" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}

Write-Host ""
Write-Host "📦 Installing dependencies..." -ForegroundColor Yellow

# Install dependencies
try {
    npm install
    if ($LASTEXITCODE -ne 0) {
        throw "npm install failed"
    }
    Write-Host "✓ Dependencies installed successfully" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to install dependencies" -ForegroundColor Red
    Write-Host "   Please check your internet connection and try again" -ForegroundColor Yellow
    Read-Host "Press Enter to exit"
    exit 1
}

Write-Host ""
Write-Host "🧪 Running quick tests..." -ForegroundColor Yellow

# Run tests
try {
    npm run test:run
    if ($LASTEXITCODE -ne 0) {
        Write-Host "⚠️  Some tests failed, but continuing..." -ForegroundColor Yellow
    } else {
        Write-Host "✓ All tests passed" -ForegroundColor Green
    }
} catch {
    Write-Host "⚠️  Tests could not run, but continuing..." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "🚀 Starting Vision Wallet..." -ForegroundColor Green
Write-Host ""
Write-Host "   Local server will start at: http://localhost:5173" -ForegroundColor Cyan
Write-Host "   Press Ctrl+C to stop the server" -ForegroundColor Yellow
Write-Host ""
Write-Host "📖 Quick Guide:" -ForegroundColor Magenta
Write-Host "   1. Click 'Enter' on the splash screen" -ForegroundColor White
Write-Host "   2. Choose a handle (username)" -ForegroundColor White
Write-Host "   3. Save your 12-word recovery phrase" -ForegroundColor White
Write-Host "   4. Start using your Vision Wallet!" -ForegroundColor White
Write-Host ""

# Start development server
try {
    npm run dev
} catch {
    Write-Host ""
    Write-Host "❌ Failed to start development server" -ForegroundColor Red
    Write-Host "   You can try running 'npm run dev' manually" -ForegroundColor Yellow
    Read-Host "Press Enter to exit"
    exit 1
}