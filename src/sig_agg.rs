//! Signature Aggregation Module
//!
//! Provides BLS12-381 signature aggregation for reducing block size and verification overhead.
//! Compatible with Ed25519 signatures (fallback mode).
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Signature type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SignatureType {
    /// Traditional Ed25519 signature (64 bytes)
    Ed25519,
    /// BLS12-381 signature (96 bytes, aggregatable)
    #[allow(clippy::upper_case_acronyms)]
    BLS,
}

/// Aggregated signature container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedSignature {
    /// Type of signatures being aggregated
    pub sig_type: SignatureType,
    /// Aggregated signature bytes (96 bytes for BLS)
    pub signature: Vec<u8>,
    /// Public keys corresponding to each signer (for verification)
    pub pubkeys: Vec<Vec<u8>>,
    /// Message(s) that were signed
    pub messages: Vec<Vec<u8>>,
}

/// BLS signature aggregation functionality
pub mod bls {

    use blst::min_pk::*;

    /// Aggregate multiple BLS signatures into one
    /// Returns the aggregated signature bytes (96 bytes)
    pub fn aggregate_signatures(signatures: &[Vec<u8>]) -> Result<Vec<u8>, String> {
        if signatures.is_empty() {
            return Err("Cannot aggregate empty signature list".to_string());
        }

        // Parse all signatures
        let mut sigs = Vec::new();
        for sig_bytes in signatures {
            if sig_bytes.len() != 96 {
                return Err(format!("Invalid BLS signature length: {}", sig_bytes.len()));
            }
            match Signature::from_bytes(sig_bytes) {
                Ok(sig) => sigs.push(sig),
                Err(e) => return Err(format!("Invalid BLS signature: {:?}", e)),
            }
        }

        // Aggregate
        let agg_sig = match AggregateSignature::aggregate(&sigs.iter().collect::<Vec<_>>(), true) {
            Ok(agg) => agg,
            Err(e) => return Err(format!("Aggregation failed: {:?}", e)),
        };

        Ok(agg_sig.to_signature().to_bytes().to_vec())
    }

    /// Verify an aggregated BLS signature
    /// pubkeys: list of public keys (each 48 bytes)
    /// messages: list of messages that were signed
    /// agg_signature: the aggregated signature (96 bytes)
    pub fn verify_aggregated(
        pubkeys: &[Vec<u8>],
        messages: &[Vec<u8>],
        agg_signature: &[u8],
    ) -> Result<bool, String> {
        if pubkeys.len() != messages.len() {
            return Err("Pubkeys and messages count mismatch".to_string());
        }
        if pubkeys.is_empty() {
            return Err("Empty pubkeys list".to_string());
        }
        if agg_signature.len() != 96 {
            return Err(format!(
                "Invalid aggregated signature length: {}",
                agg_signature.len()
            ));
        }

        // Parse signature
        let sig = match Signature::from_bytes(agg_signature) {
            Ok(s) => s,
            Err(e) => return Err(format!("Invalid signature bytes: {:?}", e)),
        };

        // Parse public keys
        let mut pks = Vec::new();
        for pk_bytes in pubkeys {
            if pk_bytes.len() != 48 {
                return Err(format!("Invalid BLS public key length: {}", pk_bytes.len()));
            }
            match PublicKey::from_bytes(pk_bytes) {
                Ok(pk) => pks.push(pk),
                Err(e) => return Err(format!("Invalid public key: {:?}", e)),
            }
        }

        // Prepare messages with DST (domain separation tag)
        const DST: &[u8] = b"VISION_BLS_SIG_V1";

        // Convert messages to slice of slices for aggregate_verify
        let msg_refs: Vec<&[u8]> = messages.iter().map(|m| m.as_slice()).collect();

        // Verify - for simplicity, we assume all messages are different
        // In production, you'd use aggregate_verify for distinct messages
        let res = sig.aggregate_verify(
            true, // messages are compressed
            &msg_refs,
            DST,
            &pks.iter().collect::<Vec<_>>(),
            true, // public keys are compressed
        );

        Ok(res == blst::BLST_ERROR::BLST_SUCCESS)
    }

    /// Generate a BLS keypair (for testing/utilities)
    /// Returns (secret_key_bytes, public_key_bytes)
    pub fn generate_keypair() -> (Vec<u8>, Vec<u8>) {
        let mut rng = rand::thread_rng();
        let mut ikm = [0u8; 32];
        use rand::RngCore;
        rng.fill_bytes(&mut ikm);

        let sk = SecretKey::key_gen(&ikm, &[]).expect("keygen");
        let pk = sk.sk_to_pk();

        (sk.to_bytes().to_vec(), pk.to_bytes().to_vec())
    }

    /// Sign a message with a BLS secret key
    pub fn sign_message(secret_key: &[u8], message: &[u8]) -> Result<Vec<u8>, String> {
        if secret_key.len() != 32 {
            return Err(format!("Invalid secret key length: {}", secret_key.len()));
        }

        let sk = match SecretKey::from_bytes(secret_key) {
            Ok(s) => s,
            Err(e) => return Err(format!("Invalid secret key: {:?}", e)),
        };

        const DST: &[u8] = b"VISION_BLS_SIG_V1";
        let sig = sk.sign(message, DST, &[]);

        Ok(sig.to_bytes().to_vec())
    }
}

/// Calculate bytes saved by aggregating N signatures
pub fn bytes_saved_by_aggregation(num_sigs: usize, sig_type: SignatureType) -> usize {
    match sig_type {
        SignatureType::Ed25519 => {
            // Ed25519: 64 bytes per sig, no aggregation benefit
            0
        }
        SignatureType::BLS => {
            // BLS: 96 bytes per sig normally, 96 bytes for aggregated
            // Savings = (N * 96) - 96 = 96 * (N - 1)
            if num_sigs > 1 {
                96 * (num_sigs - 1)
            } else {
                0
            }
        }
    }
}

/// Check if signature aggregation is enabled via environment variable
pub fn is_aggregation_enabled() -> bool {
    std::env::var("VISION_ENABLE_SIG_AGGREGATION")
        .ok()
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false)
}

/// Get minimum signatures required for aggregation to be worthwhile
pub fn min_sigs_for_aggregation() -> usize {
    std::env::var("VISION_MIN_SIGS_FOR_AGG")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(2) // Default: aggregate if 2+ signatures
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bls_aggregation() {
        // Generate keypairs
        let (sk1, pk1) = bls::generate_keypair();
        let (sk2, pk2) = bls::generate_keypair();

        let msg1 = b"transaction1";
        let msg2 = b"transaction2";

        // Sign messages
        let sig1 = bls::sign_message(&sk1, msg1).unwrap();
        let sig2 = bls::sign_message(&sk2, msg2).unwrap();

        // Aggregate signatures
        let agg_sig = bls::aggregate_signatures(&[sig1, sig2]).unwrap();

        // Verify aggregated
        let result = bls::verify_aggregated(&[pk1, pk2], &[msg1.to_vec(), msg2.to_vec()], &agg_sig);

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_bytes_saved_calculation() {
        assert_eq!(bytes_saved_by_aggregation(1, SignatureType::BLS), 0);
        assert_eq!(bytes_saved_by_aggregation(2, SignatureType::BLS), 96);
        assert_eq!(bytes_saved_by_aggregation(10, SignatureType::BLS), 864);
        assert_eq!(bytes_saved_by_aggregation(100, SignatureType::BLS), 9504);
    }
}

