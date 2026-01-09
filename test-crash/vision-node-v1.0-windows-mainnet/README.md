# Vision Node

Vision is a Rust blockchain node that combines consensus, P2P networking, HTTP APIs, and a small web UI for operating the Vision mainnet. This repository ships the full node (mining + guardian) under the Apache 2.0 license.

## Features
- Constellation/guardian roles with P2P gossip and bootstrap checkpoints
- HTTP API for wallet, mining, sync, and admin operations (Axum-based)
- Deterministic consensus parameters baked into `src/vision_constants.rs`
- Prometheus metrics and runtime tuning flags for operators

## Build & Run
- Prereqs: Rust stable toolchain, `cargo`.
- Build: `cargo build --release`
- Format/lint/tests: `cargo fmt && cargo clippy -- -D warnings && cargo test`
- Run (safe defaults): set an admin token and keep HTTP bound to localhost
  - `set VISION_ADMIN_TOKEN=your-strong-token`
  - Optional: `set VISION_HTTP_HOST=0.0.0.0` if you *explicitly* want remote HTTP access (defaults to `127.0.0.1`).
- P2P port derives from `VISION_PORT` (default 7070) and uses `VISION_PORT+2` for P2P.

## Security
- Admin endpoints require `VISION_ADMIN_TOKEN`; empty or missing tokens are rejected and logged once.
- HTTP server defaults to `127.0.0.1`. Binding to `0.0.0.0` emits a warning.
- Request bodies are capped by `VISION_MAX_BODY_BYTES` (default 256KB) and time-limited by `VISION_READ_TIMEOUT_SECS` (default 10s).
- Replace `keys.json` and `keys-recipient.json` with your own key material before running. Provided files contain placeholders only.
- See [SECURITY.md](SECURITY.md) for reporting and hardening notes.

## Governance & Community
- Please read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) and [CONTRIBUTING.md](CONTRIBUTING.md) before opening PRs.
- Forking and trademark guidance lives in [FORKING.md](FORKING.md).

## License & Trademarks
- Licensed under Apache 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
- "Vision" and related marks are trademarks of the Vision project; see the Trademark section in [FORKING.md](FORKING.md).

## Release Checklist
- A condensed OSS release checklist is in [docs/open_source_release_checklist.md](docs/open_source_release_checklist.md).
