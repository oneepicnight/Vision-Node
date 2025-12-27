# Open Source Release Checklist

- [x] License: Apache 2.0 ([LICENSE](../LICENSE)) and attribution ([NOTICE](../NOTICE)).
- [x] Security policy published ([SECURITY.md](../SECURITY.md)); admin token required by default.
- [x] Default HTTP bind set to localhost unless `VISION_HTTP_HOST` is explicitly set.
- [x] Secrets removed/redacted (`keys.json`, `keys-recipient.json` placeholders; .gitignore covers env/key files).
- [x] Placeholder key guard: Runtime detection disables signing/mining/admin when placeholders detected.
- [x] HTTP exposure warning: Startup banner logs ERROR if binding to non-localhost without admin token.
- [x] Required docs: README, CODE_OF_CONDUCT, CONTRIBUTING, FORKING, CHANGELOG.
- [x] Release and audit records: [docs/security_audit_report.md](security_audit_report.md).
- [x] CI/CD security checks: GitHub Actions workflow runs fmt, clippy, test, audit, secret scanning ([.github/workflows/security-checks.yml](../.github/workflows/security-checks.yml)).
- [x] Tooling run: `cargo fmt && cargo clippy -- -D warnings && cargo test && cargo audit` executed and documented.
- [ ] Address critical vulnerabilities: Upgrade ed25519-dalek, curve25519-dalek, maxminddb, nix (see audit report).
- [ ] Tag release and publish artifacts/binaries.
