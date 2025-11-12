feat(bech32): add diagnostic test + gate permissive fallback behind feature

Summary
- Add a small diagnostic test that computes BIP-173 hrp_expand + polymod for canonical bech32 addresses and prints results (helps compare Windows vs CI).
- Add a lightweight GitHub Actions workflow to run that diagnostic on Ubuntu with strict bech32 (no permissive fallback).
- Gate the permissive bech32 fallback behind compile-time features (`dev` or `bech32-permissive`) to avoid accidental permissive decoding in production.
- Minor clippy and lint fixes to allow CI/test runs (small edits in test helpers and probe binary).

Why
- A local Windows environment showed canonical bech32 checksum failures. The diagnostic test + CI workflow will confirm whether that mismatch is local or cross-platform.
- Feature-gating the permissive fallback prevents relaxing bech32 checks in production builds while keeping a dev escape hatch for local debugging.

What changed
- Added test: `tests/bech32_polymod.rs` — prints hrp/data lengths, polymod, and bech32::decode result for canonical addresses.
- Added workflow: `.github/workflows/bech32-strict-test.yml` — runs diagnostic test on ubuntu-latest with `BECH32_PERMISSIVE=0`.
- Gated permissive fallback in `src/crypto/addr.rs` with Cargo features: `dev` or `bech32-permissive`.
- Added `bech32-permissive` feature in `Cargo.toml`.
- Docs: README updated to document feature gating.
- Small clippy/lint fixes in a few files so CI can run with `-D warnings`.

How to test locally
- Run diagnostic test (strict):

```powershell
cargo test --test bech32_polymod --manifest-path .\Cargo.toml -- --nocapture
```

- Run with permissive fallback enabled (dev feature):

```powershell
cargo test --features dev --test bech32_polymod --manifest-path .\Cargo.toml -- --nocapture
```

- Or enable only the permissive feature:

```powershell
cargo test --features bech32-permissive --test bech32_polymod --manifest-path .\Cargo.toml -- --nocapture
```

CI expectations
- The workflow runs the diagnostic test on ubuntu-latest with `BECH32_PERMISSIVE=0`. If CI shows `polymod == 1` and `bech32::decode` succeeds there, the problem is likely Windows-local; otherwise we reproduce the issue on Linux and will need deeper investigation.

Risk & notes
- The `bech32-permissive` feature is explicitly meant for local/dev use only. Do not enable it in production builds.
- I touched a few small test/probe files to satisfy clippy warnings — no functional change beyond diagnostics and feature gates.

Checklist (for reviewers)
- [ ] Confirm the diagnostic test output in CI (Actions) and compare polymod values with local run.
- [ ] Approve gating of permissive fallback behind `dev` / `bech32-permissive`.
- [ ] Confirm docs are clear that permissive fallback is dev-only.

Suggested reviewers / labels
- Reviewers: @maintainer, @devops
- Labels: ci, tests, docs


