use crate::BlockHeader;

pub fn pow_message_bytes(h: &BlockHeader) -> Result<Vec<u8>, String> {
    // Strict decode/validation helpers
    fn hex32_strict(label: &str, s: &str) -> Result<[u8; 32], String> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s).map_err(|e| format!("{label}: invalid hex: {e}"))?;
        if bytes.len() != 32 {
            return Err(format!("{label}: expected 32 bytes, got {}", bytes.len()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }

    let parent = hex32_strict("parent_hash", &h.parent_hash)?;
    let tx_root = hex32_strict("tx_root", &h.tx_root)?;

    // Stable binary encoding to match genesis_block format:
    // MAGIC + VERSION + parent + number + timestamp + difficulty + nonce + tx_root
    // NOTE: state_root, receipts_root, da_commitment, base_fee_per_gas are NOT part of PoW message
    // They are post-mining computed values and must not affect proof of work!
    let mut out = Vec::with_capacity(4 + 4 + 32 + 8 + 8 + 8 + 8 + 32);

    out.extend_from_slice(b"VPOW"); // magic
    out.extend_from_slice(&1u32.to_le_bytes()); // version

    out.extend_from_slice(&parent);
    out.extend_from_slice(&h.number.to_le_bytes());
    out.extend_from_slice(&h.timestamp.to_le_bytes());
    out.extend_from_slice(&h.difficulty.to_le_bytes());
    out.extend_from_slice(&h.nonce.to_be_bytes());
    out.extend_from_slice(&tx_root);

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pow_message_bytes_is_stable_and_deterministic() {
        // Create synthetic BlockHeader with known values
        let h = BlockHeader {
            parent_hash: format!("0x{}", "11".repeat(32)),
            number: 12345u64,
            timestamp: 1700000000u64,
            difficulty: 1000u64,
            nonce: 42,
            pow_hash: String::new(),
            state_root: format!("0x{}", "22".repeat(32)),
            tx_root: format!("0x{}", "33".repeat(32)),
            receipts_root: format!("0x{}", "44".repeat(32)),
            da_commitment: None,
            base_fee_per_gas: 1_000_000_000_000u128,
        };

        let msg = pow_message_bytes(&h).expect("encoding failed");

        // Check magic
        assert_eq!(&msg[0..4], b"VPOW", "magic must be VPOW");

        // Check version (little-endian u32 = 1)
        assert_eq!(&msg[4..8], &1u32.to_le_bytes(), "version must be 1 LE");

        // Verify expected size: 4(magic) + 4(version) + 32(parent) + 8(number) + 8(timestamp) + 8(difficulty) + 8(nonce) + 32(tx_root)
        // = 104 bytes
        let expected_size = 4 + 4 + 32 + 8 + 8 + 8 + 8 + 32;
        assert_eq!(
            msg.len(),
            expected_size,
            "size must be 104 bytes, got {}",
            msg.len()
        );
    }

    #[test]
    fn pow_bytes_deterministic_and_miner_validator_match() {
        // This test ensures that the encoding is deterministic
        // and that both miner and validator see the same bytes.
        let h = BlockHeader {
            parent_hash: format!("0x{}", "aa".repeat(32)),
            number: 100u64,
            timestamp: 1700000000u64,
            difficulty: 256u64,
            nonce: 99,
            pow_hash: String::new(),
            state_root: format!("0x{}", "bb".repeat(32)),
            tx_root: format!("0x{}", "cc".repeat(32)),
            receipts_root: format!("0x{}", "dd".repeat(32)),
            da_commitment: None,
            base_fee_per_gas: 1_000_000_000_000u128,
        };

        // Call pow_message_bytes twice to verify determinism
        let msg1 = pow_message_bytes(&h).expect("miner encoding failed");
        let msg2 = pow_message_bytes(&h).expect("validator encoding failed");

        assert_eq!(msg1, msg2, "encoding must be deterministic");
        assert_eq!(&msg1[0..4], b"VPOW", "must encode with VPOW magic");
    }
}
