# 🌟 Vision Wallet - Quick Start Guide

A modern, secure cryptocurrency wallet built with React + TypeScript + Vite.

## 🚀 One-Click Setup

### Windows Users
**Double-click** either of these files to automatically install and start:
- `quick-start.bat` - Windows batch file
- `quick-start.ps1` - PowerShell script (may need execution policy adjustment)

### Mac/Linux Users
```bash
# Make the script executable and run
chmod +x quick-start.sh
./quick-start.sh
```

### Manual Setup (All Platforms)
```bash
# Install dependencies
npm install

# Start development server  
npm run dev

# Open http://localhost:5173 in your browser
```
- **Node.js 16+** - Download from [nodejs.org](https://nodejs.org/)
- **Internet connection** (for initial setup only)

## 🎯 Getting Started

1. **Run quick-start** - Use one of the quick-start files above
2. **Open wallet** - Navigate to http://localhost:5173
3. **Create handle** - Choose your unique username (e.g., @neo-vision)
## 🔐 Security Features

- ✅ **Local encryption** - All keys stored encrypted on your device
- ✅ **12-word recovery** - Standard BIP39 mnemonic phrases
- ✅ **No telemetry** - No data sent to third parties
- ✅ **Open source** - Full code transparency

### Notes on implementation
The project decodes Bech32/SegWit addresses using the `bech32` crate and follows BIP-0173 / BIP-0350 rules by default. A permissive fallback exists for local debugging but is gated behind compile-time features to avoid accidental use in production: compile with `--features dev` (used for local tests) or `--features bech32-permissive` to enable the permissive fallback.
## 🔧 Advanced Commands

```bash
# Development
npm run dev          # Start dev server
npm run build        # Build for production
npm run preview      # Preview production build

# Testing  
npm run test         # Run interactive tests
npm run test:run     # Run tests once

# Packaging
npm run clean        # Clean build artifacts
npm run zip          # Create distribution zip
```

## 🌐 Node Configuration

By default, the wallet connects to `http://127.0.0.1:7070` for the Vision node.

**To change the node URL:**
1. Open wallet settings (⚙️ icon in top bar)
2. Update "Node URL" field
3. Click "Test & Save"

**Supported endpoints:**
- `GET /status` - Node health check
- `GET /supply` - Token supply info
- `GET /receipts/latest` - Recent transactions
- `GET /balance/{address}` - Address balances
- `POST /tx/submit` - Submit signed transactions

## 🆘 Troubleshooting

### "Node.js not found"
- Download and install Node.js from [nodejs.org](https://nodejs.org/)
- Restart your terminal/command prompt
- Try the quick-start script again

### "npm install fails"
- Check your internet connection
- Try running: `npm install --verbose`
- Clear npm cache: `npm cache clean --force`

### "Tests fail"
- Usually safe to ignore during development
- Run `npm run test` for detailed output
- Check that all dependencies installed correctly

### "Cannot run PowerShell script"
- Right-click PowerShell → "Run as Administrator"
- Run: `Set-ExecutionPolicy -ExecutionPolicy RemoteSigned`
- Try the script again

## 🎮 Enter Vision Portal

The wallet includes deep-link integration for the Vision game world:

- **Protocol handler**: `vision://enter?address=<addr>&handle=<handle>`
- **Fallback URL**: `http://127.0.0.1:5173/vision`

## 📁 Project Structure

```
vision-wallet/
├── src/
│   ├── routes/          # App pages (Splash, Home, Settings)
│   ├── lib/             # Core utilities (keystore, api, guards)  
│   ├── state/           # Zustand stores (wallet, onboarding)
│   └── __tests__/       # Test suites
├── quick-start.*        # One-click setup scripts
└── package.json         # Dependencies and scripts
```

## 🔄 Updating

To update to a newer version:
1. Download the latest release
2. Run the quick-start script again
3. Your wallet data is preserved (stored in browser)

## 💾 Backup & Recovery

**Always backup your 12-word recovery phrase!**

- **Export backup**: Settings → Security → "Export Backup"
- **Restore wallet**: Use your 12 words on any device
- **Wipe wallet**: Settings → Danger Zone (type "VISION" to confirm)

---

**Need help?** Check the project documentation or create an issue on GitHub.

**Ready to start?** Double-click `quick-start.bat` (Windows) or run `npm run dev` manually!

## Dev quickstart (Windows PowerShell)

1) Start Vision node

.\vision-node.exe

2) Start market backend (Rust wallet server)

$env:STRIPE_SECRET="sk_test_xxx"
$env:STRIPE_WEBHOOK_SECRET="whsec_xxx"
cargo run

3) Start wallet UI with proxy to market

$env:VITE_VISION_NODE_URL="http://127.0.0.1:7070"
$env:VITE_VISION_MARKET_URL="/api/market"
$env:VITE_FEATURE_DEV_PANEL="true"
$env:VITE_ADMIN_TOKEN="devtoken123"
npm i
npm run dev -- --host 127.0.0.1 --port 4173

Open http://127.0.0.1:4173 — use "Market" to buy / "Orders" to inspect.

Bonus: Add a QR code on each LAND listing card for pay_to using qrcode.react and a copy-to-clipboard button for the crypto address.

### Lints
- Run clippy across all targets:
	`cargo clippy --all-targets --all-features -q`
- Auto-fix trivial imports:
	`cargo fix --allow-dirty --allow-no-vcs`

## Windows testing notes (sled & routes)

**Avoid double-opening sled paths.**
- The server and tests must not open the same DB path concurrently.
- Use the provided `cash_store::db_owned()` helper (which centralizes opening),
	or per-test temp dirs (we use `tempfile::TempDir`) to isolate DBs.

**Axum routes on 0.7 use `{id}` captures.**
- Older `":id"` syntax will panic on startup.
- All routes using path params should be like `/market/land/listings/{id}`.

**Running tests safely**
- Unit tests: `cargo test -q`
- E2E tests that spawn a server: prefer reading state via HTTP endpoints,
	not by opening the sled DB directly (prevents Windows lock contention).

## Dev feature & CI behavior

### `dev` Cargo feature
We gate test/dev helpers behind a feature so release builds stay clean:
- Enable in local tests/dev:
	- Linux/macOS:
		```bash
		STRIPE_TEST_NO_VERIFY=1 ADMIN_TOKEN=devtoken123 cargo test --all --all-features -q
		```
	- Windows (sled-safe single thread):
		```powershell
		$env:STRIPE_TEST_NO_VERIFY="1"
		$env:ADMIN_TOKEN="devtoken123"
		cargo test --all --all-features -- --test-threads=1
		```

### CI (GitHub Actions)
The workflow runs on **Ubuntu & Windows**:
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test` (Windows uses single-thread to avoid sled lock contention)
- Stripe webhooks are simulated with `STRIPE_TEST_NO_VERIFY=1`
 
### Address parsing permissive feature

We include a narrowly-scoped permissive fallback for address parsing that
handles an off-by-one padding artifact that can appear during 5→8 bit
conversions on some platforms or libraries. This fallback is intentionally
gated behind compile-time features so production builds remain strict by
default.

- Enable permissive parsing only for local development or CI diagnostic runs:
	- `--features dev` (already used for a set of dev helpers)
	- or `--features bech32-permissive` to enable only the Bech32/CashAddr
		permissive trimming behavior.

- Recommended usage:
	- CI (diagnostics): enable `bech32-permissive` on a neutral runner (e.g.
		Ubuntu) when investigating platform-specific checksum/convert issues.
	- Production releases: do NOT enable this feature. Keep parsing strict to
		avoid accepting malformed addresses.

Examples:

```bash
# Run the BCH CashAddr focused test with permissive parsing enabled
cargo test --test scripthash_bch_cashaddr --features bech32-permissive -- --nocapture

# Run all tests with dev features (local only)
cargo test -j 1 --features dev
```
### Windows testing note (sled)
Avoid opening the same sled path twice from different processes.
Our tests/e2e access state via HTTP endpoints and/or per-test temp dirs.

## Git bootstrap (optional)
If this folder isn't yet a git repo, you can create one and make a first commit:

PowerShell

```powershell
git init
git add -A
git commit -m "chore: enable dev feature gating, add CI, clippy gates, and docs"
git branch -M main
```

bash

```bash
git init
git add -A
git commit -m "chore: enable dev feature gating, add CI, clippy gates, and docs"
git branch -M main
```

## Confirmations & Watchers

### Exact confirmations
Watchers compute confirmations precisely:
`confirmations = tip_height - tx_height + 1`
We obtain `tip_height` via `blockchain.headers.subscribe`. Thresholds come from `vision.toml` or `CONF_*` env.

### Scripthash fast path
We query Electrum with `blockchain.scripthash.get_history` when we can derive a **scriptPubKey** from the invoice address:
- **BTC**: legacy Base58 **P2PKH** and Bech32 SegWit v0 (P2WPKH/P2WSH) supported ✅
- **BCH**: **CashAddr P2PKH/P2SH** ✅ and legacy **Base58 P2PKH** ✅
- **DOGE**: legacy Base58 **P2PKH** supported

If an address can’t be parsed into a script, we fall back to `address.get_history` and then an HTTP explorer.

### Mock-chain mode (zero-network demos)
- Enable: `MOCK_CHAIN=1` (or `[test].mock_chain = true` in `vision.toml`)
- Any listing with `pay_to` starting with `mock:` or `demo:` is **confirmed immediately**
- Confirm callback target: `MARKET_CONFIRM_URL=http://127.0.0.1:<port>/_market/land/confirm`

### Electrum plaintext (tests)
- `ELECTRUM_PLAINTEXT=1` allows plaintext Electrum for local mocks in tests.
