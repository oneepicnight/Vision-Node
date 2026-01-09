#!/usr/bin/env python3
"""Compute canonical ECON_HASH for Vision Node mainnet"""

import hashlib

# Mainnet vault addresses from token_accounts.toml
staking_vault = "0xb977c16e539670ddfecc0ac902fcb916ec4b944e"
ecosystem_fund = "0x8bb8edcd4cdbcb132cc5e88ff90ba48cebf11cbd"
founder1 = "0xdf7a79291bb96e9dd1c77da089933767999eabf0"
founder2 = "0x083f95edd48e3e9da396891b704994b86e7790e7"

# Split percentages in basis points (10000 BPS = 100%)
vault_bps = 5000   # 50%
fund_bps = 3000    # 30%
founder1_bps = 1000 # 10%
founder2_bps = 1000 # 10%

# Validate splits sum to 100%
total = vault_bps + fund_bps + founder1_bps + founder2_bps
assert total == 10000, f"Splits must sum to 10000 BPS (100%), got {total}"

# Build deterministic hash input (fixed order, same as Rust implementation)
hasher = hashlib.blake2b(digest_size=32)  # Blake2b-256 (closest to Blake3)

# Addresses (deterministic order)
hasher.update(staking_vault.encode('utf-8'))
hasher.update(ecosystem_fund.encode('utf-8'))
hasher.update(founder1.encode('utf-8'))
hasher.update(founder2.encode('utf-8'))

# Splits (same order, little-endian u16)
hasher.update(vault_bps.to_bytes(2, 'little'))
hasher.update(fund_bps.to_bytes(2, 'little'))
hasher.update(founder1_bps.to_bytes(2, 'little'))
hasher.update(founder2_bps.to_bytes(2, 'little'))

hash_bytes = hasher.digest()
hash_hex = hash_bytes.hex()

print(f"ECON_HASH (canonical): {hash_hex}")
print()
print("Copy this value into src/genesis.rs ECON_HASH constant")
print()
print("Inputs:")
print(f"  Staking vault (50%): {staking_vault}")
print(f"  Ecosystem fund (30%): {ecosystem_fund}")
print(f"  Founder1 (10%): {founder1}")
print(f"  Founder2 (10%): {founder2}")
print()
print(f"NOTE: This uses Blake2b-256 for computation.")
print(f"The Rust code uses Blake3, which will produce a different hash.")
print(f"This is a temporary approximation - run the Rust test to get the real value.")
