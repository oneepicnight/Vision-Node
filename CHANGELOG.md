# Changelog

## v1.0.2 (2026-01-11) - Hotfix

### Critical Fixes
- **Fixed genesis hash compatibility**: Reverted to v1.0.0 genesis hash (`d6469ec95f56b56be4921ef40b9795902c96f2ad26582ef8db8fac46f4a7aa13`) - miner field is backward compatible with `#[serde(default)]`, no network reset required
- **Fixed wallet asset loading**: Corrected wallet deployment structure to preserve `assets/` subdirectory (fixes JavaScript module MIME type errors)

### Enhanced Diagnostics
- **Startup genesis logging**: Added `[GENESIS]` and `[CHAIN]` logs showing compiled vs database genesis hashes for troubleshooting
- **Enhanced handshake rejection logs**: Show local_compiled, local_active, and remote genesis with actionable debugging hints
- **Ghost node diagnostic script**: Added `diagnose-ghost-node.ps1` to identify wrong/old vision-node.exe processes

### Infrastructure
- Confirmed P2P binding to `0.0.0.0:7072` by default (accepts connections from all network interfaces)
- Distribution packages updated with corrected genesis and diagnostic tools

## v1.0.1 (2026-01-10)

### Major Features
- **Miner Identity Propagation**: Block rewards now go to actual miner addresses with full P2P propagation
- **Consensus Quorum Validation**: Fixed critical security vulnerability in sync/mining/exchange gates to validate chain compatibility
- **Operational Visibility**: Added periodic quorum logging (30s) and API endpoint fields for debugging
- **Mining Button Persistence**: Fixed "Make Fans Go Brrrrr" button state across tab switches

### Security Fixes
- **CRITICAL**: All gates (sync/mining/exchange) now check `compatible_peers` instead of raw TCP connection count
- Risk eliminated: Nodes can no longer sync/mine/trade with peers on different forks/chains

### Platform Support
- **Linux GNU Compatibility**: Added probestack shim for Linux GNU linker
- Enhanced `install.sh` with mandatory dependency checks

## v1.0.0
- Initial public release of Vision Node under Apache 2.0.
- Added security hardening: admin token warning when unset, HTTP defaults to localhost with explicit opt-in for remote binding.
- Redacted baked-in key files and documented secure defaults.
- Added OSS release artifacts: LICENSE, SECURITY.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md, FORKING.md, README.md, NOTICE alignment.
- Added release checklist and security audit report stubs.
