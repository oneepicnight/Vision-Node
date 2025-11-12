#!/bin/bash

# Vision Wallet - Quick Start Script (Mac/Linux)
# This script will install dependencies and start the development server

echo ""
echo "🌟 Vision Wallet - Quick Start Setup"
echo "==================================="
echo ""

# Check if Node.js is installed
if ! command -v node &> /dev/null; then
    echo "❌ Node.js not found. Please install Node.js from https://nodejs.org/"
    echo "   Minimum required version: Node.js 16+"
    read -p "Press Enter to exit"
    exit 1
fi

NODE_VERSION=$(node --version)
echo "✓ Node.js found: $NODE_VERSION"

# Check if npm is available
if ! command -v npm &> /dev/null; then
    echo "❌ npm not found. Please reinstall Node.js"
    read -p "Press Enter to exit"
    exit 1
fi

NPM_VERSION=$(npm --version)
echo "✓ npm found: v$NPM_VERSION"

echo ""
echo "📦 Installing dependencies..."

# Install dependencies
if ! npm install; then
    echo "❌ Failed to install dependencies"
    echo "   Please check your internet connection and try again"
    read -p "Press Enter to exit"
    exit 1
fi

echo "✓ Dependencies installed successfully"

echo ""
echo "🧪 Running quick tests..."

# Run tests
if npm run test:run; then
    echo "✓ All tests passed"
else
    echo "⚠️  Some tests failed, but continuing..."
fi

echo ""
echo "🚀 Starting Vision Wallet..."
echo ""
echo "   Local server will start at: http://localhost:5173"
echo "   Press Ctrl+C to stop the server"
echo ""
echo "📖 Quick Guide:"
echo "   1. Click 'Enter' on the splash screen"
echo "   2. Choose a handle (username)"
echo "   3. Save your 12-word recovery phrase"
echo "   4. Start using your Vision Wallet!"
echo ""

# Start development server
npm run dev