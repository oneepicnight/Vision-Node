# SHA256 Hash Lock Quick Reference

**Version**: v1.0.0 mainnet  
**Last Updated**: December 26, 2025

---

## TL;DR

✅ **HTLC hash locks = SHA256** (cross-chain standard)  
✅ **htlc_id = BLAKE3** (internal identifier only)  
✅ **All hash lock ops → `src/swap/hashlock.rs`**  
✅ **CI guard: `check-htlc-hashlock-safety.ps1`**

---

## Core Functions

### Create Hash Lock

```rust
use crate::swap::htlc_hash_lock_hex;

let secret = "my_secret_preimage_12345";
let hash_lock = htlc_hash_lock_hex(secret.as_bytes());
// Returns: "a1b2c3d4..." (64-char hex SHA256)
```

### Verify Preimage

```rust
use crate::swap::verify_hash_lock_hex;

let preimage = "my_secret_preimage_12345";
let stored_hash = "a1b2c3d4..."; // From database

if verify_hash_lock_hex(preimage.as_bytes(), stored_hash) {
    println!("Valid preimage!");
}
```

---

## File Locations

| File                              | Purpose                          |
|-----------------------------------|----------------------------------|
| `src/swap/hashlock.rs`            | SHA256 hash lock primitives      |
| `src/main.rs` (claim_htlc)        | Preimage verification            |
| `src/atomic_swaps.rs`             | Swap state machine               |
| `check-htlc-hashlock-safety.ps1`  | CI safety verification           |

---

## CI Command

```powershell
powershell -ExecutionPolicy Bypass -File check-htlc-hashlock-safety.ps1
```

**Exit 0** = Pass | **Exit 1** = BLAKE3 detected in hash lock path

---

## Test Vectors

```rust
// Empty string
SHA256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855

// "abc"
SHA256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad

// "The quick brown fox jumps over the lazy dog"
SHA256("fox...") = d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592
```

---

## Why SHA256?

| Blockchain   | Hash Function | Standard     |
|--------------|---------------|--------------|
| Bitcoin      | SHA256        | BIP-199      |
| Ethereum     | SHA256        | ERC-20 HTLC  |
| Lightning    | SHA256        | BOLT-3       |
| BCH          | SHA256        | Native       |
| DOGE         | SHA256        | Native       |

**Using anything else breaks cross-chain atomic swaps.**

---

## Rules

### ✅ DO

- Use `htlc_hash_lock_hex()` for hash locks
- Use `verify_hash_lock_hex()` for verification
- Document BLAKE3 as "internal identifier only"
- Run CI guard before merging

### ❌ DON'T

- Use `blake3::hash()` for hash locks
- Bypass `swap::verify_hash_lock_hex()`
- Implement custom SHA256 for HTLCs
- Skip CI safety check

---

## Build Commands

```bash
# Dev build
cargo build

# Release build
cargo build --release

# CI safety check
powershell -ExecutionPolicy Bypass -File check-htlc-hashlock-safety.ps1
```

---

## Status

| Check                     | Result |
|---------------------------|--------|
| SHA256 module exists      | ✅     |
| Unit tests pass           | ✅     |
| CI guard passes           | ✅     |
| Release build succeeds    | ✅     |
| BLAKE3 isolated           | ✅     |
| Cross-chain compatible    | ✅     |

**Release**: vision-node.exe (38.9 MB, Dec 26 2025 10:28 AM)

---

## Help

**Full Documentation**: `SHA256_HASH_LOCK_IMPLEMENTATION.md`

**Key Files**:
- Hash lock crypto: [src/swap/hashlock.rs](src/swap/hashlock.rs)
- Claim verification: [src/main.rs](src/main.rs#L27039)
- Swap state machine: [src/atomic_swaps.rs](src/atomic_swaps.rs)
- CI safety guard: [check-htlc-hashlock-safety.ps1](check-htlc-hashlock-safety.ps1)
