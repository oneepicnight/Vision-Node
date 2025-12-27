# Security Audit Report

Date: 2025-12-26
Scope: vision-node (Rust HTTP/P2P node + tooling)

## Summary
- Secrets: Redacted baked-in keypairs and expanded .gitignore to block accidental commits of env/key material.
- Defaults: HTTP binds to localhost by default; admin endpoints stay disabled without `VISION_ADMIN_TOKEN` and log once.
- Limits: Request body cap (256KB default) and 10s request timeout already applied globally.
- OSS hygiene: Added Apache-2.0 licensing metadata and release docs.
- Placeholder Key Guard: Added runtime detection and tripwire to disable sensitive operations when placeholder/missing keys detected.
- CI/CD Security: Added GitHub Actions workflow for continuous security checks (fmt, clippy, test, audit, secret scanning).

## Findings & Actions
- **Secret files removed**: Replaced embedded keys with placeholders so operators must supply their own material ([keys.json](../keys.json#L1-L4), [keys-recipient.json](../keys-recipient.json#L1-L4)).
- **Ignore sensitive artifacts**: Added ignores for env/keys ([.gitignore](../.gitignore#L12-L17)).
- **Admin token enforcement**: If `VISION_ADMIN_TOKEN` is missing, admin endpoints stay disabled and emit a single warning ([src/main.rs](../src/main.rs#L28554-L28593)).
- **HTTP bind tightened**: Default host is `127.0.0.1`; insecure `0.0.0.0` requires explicit `VISION_HTTP_HOST` and logs a warning.
- **HTTP exposure warning**: Added startup check that logs ERROR banner if binding to non-localhost without `VISION_ADMIN_TOKEN` set ([src/main.rs](../src/main.rs#L6062-L6077)).
- **Placeholder key detection**: Added runtime guard module that checks keys.json and keys-recipient.json for placeholder/missing/short keys ([src/placeholder_keys.rs](../src/placeholder_keys.rs#L1-L180)).
  - **Startup banner**: Logs ERROR banner with detailed remediation steps when placeholders detected ([src/main.rs](../src/main.rs#L6034-L6060)).
  - **Operation guards**: Disables signing ([src/main.rs](../src/main.rs#L11023-L11035)), mining ([src/main.rs](../src/main.rs#L1258-L1270)), and admin endpoints ([src/main.rs](../src/main.rs#L28553-L28556)) when placeholders detected.
- **License metadata**: Added `license = "Apache-2.0"` to the crate manifest ([Cargo.toml](../Cargo.toml#L7)).
- **Request limits already present**: Global middleware enforces Content-Length checks and a per-request timeout; configurable via env (`VISION_MAX_BODY_BYTES`, `VISION_READ_TIMEOUT_SECS`).
- **P2P size/timeouts**: Handshake capped at 10KB with a 12s timeout ([src/p2p/connection.rs](../src/p2p/connection.rs#L79-L123)).
- **CI/CD workflow**: Added GitHub Actions security-checks.yml with format check, clippy lint, test suite, cargo audit, secret scanning (TruffleHog + Gitleaks), and placeholder key detection ([.github/workflows/security-checks.yml](../.github/workflows/security-checks.yml#L1-L170)).

## Potential Risks / Follow-ups
- Validate that all admin/privileged routes consistently call `check_admin`; spot-checking showed coverage but should be exhaustively verified after refactors.
- Replace placeholder key files with operator-generated keys before any mainnet/testnet deployment.
- Address clippy warnings: 86 errors reported, mostly unused code and style issues across miner, swap, p2p, and market modules. Consider targeted `#[allow]` attributes or cleanup passes.
- Fix test failure: examples/test_simd.rs has unresolved crate dependency (`vision_node`). Need to add proper path or move to tests/.
- Dependency vulnerabilities: See Audit Results section below for details on 5 vulnerabilities and 6 unmaintained dependency warnings.

## Tooling Outputs

### cargo fmt
**Status**: ✅ PASS (after whitespace cleanup)

Initial run failed on trailing whitespace in:
- src/miner/hint_manager.rs
- src/p2p/beacon_bootstrap.rs
- src/p2p/upnp.rs

After cleanup: All files formatted correctly.

### cargo clippy -- -D warnings
**Status**: ❌ FAIL (86 errors)

Major issues:
- **Unused code**: Many unused functions, variables, imports across miner/manager.rs, swap modules, p2p, market
- **Manual clamp patterns**: Several instances of manual min/max chains that could use `.clamp()`
- **Redundant patterns**: Unnecessary `mut` bindings, redundant closures
- **Type complexity**: Some complex type signatures (should_be_lint)

**Recommendation**: 
1. Add targeted `#[allow(dead_code)]` for staged modules not yet fully wired
2. Fix legitimate style issues (clamp, redundant patterns)
3. Remove truly unused items in active code paths

Full output summary:
- miner/manager.rs: ~15 unused function warnings
- atomic_swap/*.rs: ~20 unused struct/function/import warnings  
- p2p/*.rs: ~10 unused function/variable warnings
- market/*.rs: ~12 unused function warnings
- bin/*.rs: ~5 pattern/style warnings
- examples/: ~8 various warnings
- src/main.rs: ~16 unused warnings + style issues

### cargo test
**Status**: ❌ FAIL

Primary failure:
```
error[E0433]: failed to resolve: could not find `vision_node` in the list of imported crates
  --> examples/test_simd.rs:1:5
```

The example file tries to reference `vision_node` crate but doesn't have proper path configuration.

Additional warnings: Multiple unused item warnings (consistent with clippy findings).

**Recommendation**: Move examples/test_simd.rs to tests/ with proper crate imports or add [[example]] entry to Cargo.toml.

### cargo audit
**Status**: ⚠️ 5 VULNERABILITIES + 6 WARNINGS

#### Critical/High Vulnerabilities:

1. **curve25519-dalek** (RUSTSEC-2024-0344)
   - Issue: Timing variability in Montgomery ladder implementation
   - Impact: Potential side-channel attack on scalar multiplication
   - Fix: Upgrade to patched version
   - Used by: ed25519-dalek, x25519-dalek chains

2. **ed25519-dalek** (RUSTSEC-2022-0093)
   - Issue: Double public key signing function can create malicious signatures
   - Impact: Signature forgery possible under specific conditions
   - Fix: Upgrade to 2.0+ which removes vulnerable function
   - Direct dependency: Core signing operations

3. **maxminddb** (RUSTSEC-2025-0132)
   - Issue: Potential vulnerability in database parsing
   - Impact: TBD (recently disclosed)
   - Fix: Upgrade to latest version
   - Used by: GeoIP lookup functionality

4. **nix** (RUSTSEC-2021-0119)
   - Issue: Out-of-bounds write in `nix::unistd::getgrouplist`
   - Impact: Memory corruption possible
   - Fix: Upgrade to 0.23.2+
   - Transitive dependency

5. **ruint** (RUSTSEC-2025-0137)
   - Issue: Recently disclosed vulnerability
   - Impact: TBD
   - Fix: **NO FIX AVAILABLE YET**
   - Used by: alloy-primitives chain

#### Unmaintained Dependencies (Warnings):

1. **derivative** - No longer maintained
2. **fxhash** - No longer maintained  
3. **instant** - No longer maintained
4. **mach** - No longer maintained
5. **paste** - No longer maintained (consider paste-ors fork)
6. **proc-macro-error** - No longer maintained

**Immediate Actions Required**:
- Upgrade ed25519-dalek to 2.0+ (critical for signature security)
- Upgrade curve25519-dalek to patched version
- Upgrade maxminddb and nix to latest secure versions
- Monitor ruint advisory for available fix and upgrade ASAP
- Consider replacing unmaintained dependencies with maintained alternatives

**Security Impact**:
- Signature operations currently using vulnerable crypto libraries
- GeoIP and system calls using libraries with known issues
- Unmaintained dependencies may accumulate undiscovered vulnerabilities

### Secret Scanning
**Status**: ✅ CLEAN (from prior rg scans)

No production credentials detected. All matches were:
- Test/example key files (keys.json, keys-recipient.json with placeholders)
- Package-lock integrity hashes
- Test vectors in code comments

## Scan Notes
- `rg` searches for tokens/secrets found only template/test data (keys fixtures, env templates). No production credentials detected.
- JWT-like and base64 matches were limited to test vectors and package-lock integrity hashes.

## Commands Run
```bash
# Format check
cargo fmt

# Lint check  
cargo clippy -- -D warnings

# Test suite
cargo test

# Security audit
cargo audit

# Secret scanning
rg -n "ADMIN_TOKEN|secret|mnemonic|private_key|bearer|authorization|api_key|password|seed" .
```

## Next Steps
1. **Priority 1 (Critical)**: Upgrade ed25519-dalek and curve25519-dalek for signature security
2. **Priority 2 (High)**: Fix test_simd.rs dependency issue; address clippy unused code in production paths
3. **Priority 3 (Medium)**: Upgrade maxminddb, nix; replace unmaintained dependencies
4. **Priority 4 (Low)**: Clean up clippy style warnings for better code hygiene
5. **Ongoing**: Monitor ruint advisory and upgrade when fix available; run CI security checks on every PR
