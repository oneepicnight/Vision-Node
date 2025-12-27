# Security Policy

## Reporting
- Email security issues privately to security@vision.dev.
- Include reproduction steps, impacted components, and any logs you can safely share.
- Please allow 72 hours for an initial response.

## Scope & Expectations
- All binaries and crates in this repo, plus the HTTP, P2P, and consensus layers.
- Do **not** test against production nodes without consent. Prefer local or test deployments.

## Hardening Defaults
- Admin APIs require `VISION_ADMIN_TOKEN`; if unset, admin routes are disabled and a warning is logged once.
- HTTP server binds to `127.0.0.1` by default. Use `VISION_HTTP_HOST=0.0.0.0` only when you understand the exposure.
- Request limits: `VISION_MAX_BODY_BYTES` (default 256KB) and `VISION_READ_TIMEOUT_SECS` (default 10s).
- Seed export/import is opt-in via env flags (`VISION_ALLOW_SEED_EXPORT`, `VISION_ALLOW_SEED_IMPORT`).
- Replace placeholder key files (`keys.json`, `keys-recipient.json`) with your own keys before running.

## Vulnerability Handling
- We will acknowledge receipt, triage, and provide a remediation or mitigation timeline when possible.
- Please avoid public disclosure until a fix is available.

## Third-Party Dependencies
- We run `cargo audit` and `cargo clippy` during releases. Known issues are documented in `docs/security_audit_report.md`.
