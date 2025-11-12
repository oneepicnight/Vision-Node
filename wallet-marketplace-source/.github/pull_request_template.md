## What
- …

## Why
- …

## Test Plan
- [ ] cargo clippy --all-targets --all-features (no warnings)
- [ ] cargo test (Windows single-thread: -- --test-threads=1)

## Notes
- STRIPE_TEST_NO_VERIFY=1 used in tests
- ADMIN_TOKEN required for admin endpoints
