# Changelog

## 2025-10-23 — bech32 diagnostic + feature gating

Summary
- Add a diagnostic test that computes BIP-173 hrp_expand + polymod for canonical Bech32 addresses and logs the results. This helps compare the local Windows environment with CI runners.
- Add a lightweight GitHub Actions workflow to run the diagnostic test on Ubuntu with strict Bech32 decoding (permissive disabled) so the issue can be reproduced or ruled out on CI.
- Gate the permissive Bech32 fallback behind compile-time features (`dev` or `bech32-permissive`). Production builds without these features remain strict-only.

Why
- A Windows development environment showed InvalidChecksum for canonical Bech32 addresses. The diagnostic test plus CI run will determine whether the checksum mismatch is environment-local or cross-platform.

Impact
- Default behavior unchanged for production builds.
- Developers can enable permissive fallback locally by building with `--features dev` (existing dev feature) or `--features bech32-permissive`.

How to test locally
- Strict-only (default):
  - `cargo test --test bech32_polymod -- --nocapture`
- With permissive fallback (dev):
  - `cargo test --features dev --test bech32_polymod -- --nocapture`

Notes
- The permissive fallback is intended as a dev escape hatch only. Do not enable `bech32-permissive` in production builds.
## [Unreleased]
### Fixed
- Windows sled DB lock conflicts by centralizing DB access via `cash_store::db_owned()` and avoiding concurrent opens in tests.
- Axum route captures updated to `{id}` to match axum 0.7, preventing startup panics.
- E2E tests read state via API instead of opening sled directly (Windows-safe).

### Added
- `market_cash_orders` sled tree as the canonical CASH order store.
- Migration shim `migrate_legacy_prefix()` (+ optional `cleanup_legacy_prefix()` behind `CASH_MIGRATION_DELETE_LEGACY=1`).
- Cursor pagination for `/admin/cash/orders` with base64 `<updated_at>|<id>` cursors.
- `market::cursor::{encode_cursor, decode_cursor}` with round-trip unit test.
- Startup logs show absolute sled DB path.

### Dev / DX
- Clippy config & lint gating; README lints section.
- Windows testing notes: sled patterns and route syntax.

### PR checklist
- [ ] `cargo build` (dev) passes
- [ ] `cargo clippy --all-targets --all-features` passes (or warnings acknowledged)
- [ ] `cargo test` passes (Windows + Linux)
- [ ] Verified migration logs on first boot (moved count)
- [ ] (Optional) Set `CASH_MIGRATION_DELETE_LEGACY=1` and confirmed legacy cleanup
- [ ] Frontend Orders uses cursor pagination; “Load more” works
