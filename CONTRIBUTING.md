# Contributing

Thanks for improving Vision! Before you open a PR:

1. Read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
2. Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test`.
3. Avoid adding secrets or private endpoints. Never hard-code admin tokens or keys.
4. Keep production-safe defaults; gate experimental features behind env flags (e.g., `VISION_DEV=1`).
5. Include tests for consensus-critical or security-sensitive changes.

## Pull Requests
- Describe the change, risks, and rollout steps.
- Reference any relevant issues or security tickets.
- Small, focused PRs are easier to review and land.

## Licensing
- Contributions are licensed under Apache 2.0. By submitting a PR you agree to that licensing.
