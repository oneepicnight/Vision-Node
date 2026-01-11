// Miners Vault Multisig Addresses - Fee Collection (Non-Custodial)
// Generates multisig addresses for collecting mining fees ONLY
// NO seeds stored - only public keys from config
// Requires offline signing with private keys held by trusted guardians

use anyhow::{anyhow, Result};
use bitcoin::blockdata::script::Script;
use bitcoin::secp256k1::PublicKey;
use bitcoin::util::address::{Address, Payload};
use bitcoin::{Network, PublicKey as BitcoinPublicKey};
use sha2::{Digest, Sha256};

/// Multisig configuration from environment
#[derive(Debug, Clone)]
pub struct MultisigConfig {
    /// Threshold (m in m-of-n)
    pub m: usize,
    /// Public keys (hex-encoded compressed secp256k1 pubkeys)
    pub pubkeys: Vec<String>,
}

impl MultisigConfig {
    /// Load from environment variables:
    /// - VISION_MINERS_MULTISIG_M: threshold (e.g., "2" for 2-of-3)
    /// - VISION_MINERS_MULTISIG_PUBKEYS: comma-separated hex pubkeys
    pub fn from_env() -> Result<Self> {
        let m_str = std::env::var("VISION_MINERS_MULTISIG_M").unwrap_or_else(|_| "2".to_string());
        let m: usize = m_str
            .parse()
            .map_err(|e| anyhow!("Invalid VISION_MINERS_MULTISIG_M: {}", e))?;

        let pubkeys_str = std::env::var("VISION_MINERS_MULTISIG_PUBKEYS")
            .unwrap_or_else(|_| {
                // Default test pubkeys (DO NOT USE IN PRODUCTION)
                tracing::warn!("⚠️  Using DEFAULT test pubkeys - SET VISION_MINERS_MULTISIG_PUBKEYS in production!");
                "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5,\
                 03774ae7f858a9411e5ef4246b70c65aac5649980be5c17891bbec17895da008cb,\
                 0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798".to_string()
            });

        let pubkeys: Vec<String> = pubkeys_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if pubkeys.len() < m {
            return Err(anyhow!(
                "Multisig requires at least {} pubkeys, got {}",
                m,
                pubkeys.len()
            ));
        }

        if m == 0 {
            return Err(anyhow!("Multisig threshold must be >= 1"));
        }

        Ok(MultisigConfig { m, pubkeys })
    }

    /// Get n (total number of pubkeys)
    pub fn n(&self) -> usize {
        self.pubkeys.len()
    }
}

/// Parse hex pubkey string to secp256k1 PublicKey
fn parse_pubkey(hex: &str) -> Result<PublicKey> {
    let bytes = hex::decode(hex).map_err(|e| anyhow!("Invalid hex pubkey: {}", e))?;

    PublicKey::from_slice(&bytes).map_err(|e| anyhow!("Invalid secp256k1 pubkey: {}", e))
}

/// Build P2SH multisig redeem script
/// Format: m <pubkey1> <pubkey2> ... <pubkeyn> n OP_CHECKMULTISIG
fn build_multisig_redeem_script(config: &MultisigConfig) -> Result<Script> {
    use bitcoin::blockdata::opcodes;
    use bitcoin::blockdata::script::Builder;

    // Convert pubkeys
    let mut pubkeys_parsed = Vec::new();
    for hex in &config.pubkeys {
        let pk = parse_pubkey(hex)?;
        let btc_pk = BitcoinPublicKey {
            compressed: true,
            inner: pk,
        };
        pubkeys_parsed.push(btc_pk);
    }

    // Sort pubkeys (BIP67 lexicographic ordering for determinism)
    pubkeys_parsed.sort_by(|a, b| {
        let a_bytes = a.to_bytes();
        let b_bytes = b.to_bytes();
        a_bytes.cmp(&b_bytes)
    });

    // Build script: m <pk1> <pk2> ... <pkn> n CHECKMULTISIG
    let mut builder = Builder::new();

    // Push m (threshold)
    builder = match config.m {
        1 => builder.push_opcode(opcodes::all::OP_PUSHNUM_1),
        2 => builder.push_opcode(opcodes::all::OP_PUSHNUM_2),
        3 => builder.push_opcode(opcodes::all::OP_PUSHNUM_3),
        4 => builder.push_opcode(opcodes::all::OP_PUSHNUM_4),
        5 => builder.push_opcode(opcodes::all::OP_PUSHNUM_5),
        _ => return Err(anyhow!("Multisig threshold must be 1-5")),
    };

    // Push public keys
    for pk in &pubkeys_parsed {
        builder = builder.push_key(pk);
    }

    // Push n (total keys)
    builder = match pubkeys_parsed.len() {
        1 => builder.push_opcode(opcodes::all::OP_PUSHNUM_1),
        2 => builder.push_opcode(opcodes::all::OP_PUSHNUM_2),
        3 => builder.push_opcode(opcodes::all::OP_PUSHNUM_3),
        4 => builder.push_opcode(opcodes::all::OP_PUSHNUM_4),
        5 => builder.push_opcode(opcodes::all::OP_PUSHNUM_5),
        _ => return Err(anyhow!("Multisig n must be 1-5")),
    };

    // CHECKMULTISIG
    builder = builder.push_opcode(opcodes::all::OP_CHECKMULTISIG);

    Ok(builder.into_script())
}

/// Generate BTC P2SH multisig address (3... for mainnet)
pub fn generate_btc_multisig_address(config: &MultisigConfig) -> Result<String> {
    let redeem_script = build_multisig_redeem_script(config)?;
    let address = Address::p2sh(&redeem_script, Network::Bitcoin)
        .map_err(|e| anyhow!("Failed to create P2SH address: {}", e))?;

    Ok(address.to_string())
}

/// Generate BCH P2SH multisig address (bitcoincash:p... for CashAddr)
/// BCH uses same P2SH construction as BTC, but encodes in CashAddr format
pub fn generate_bch_multisig_address(config: &MultisigConfig) -> Result<String> {
    let redeem_script = build_multisig_redeem_script(config)?;

    // Create P2SH address (Bitcoin style first)
    let address = Address::p2sh(&redeem_script, Network::Bitcoin)
        .map_err(|e| anyhow!("Failed to create P2SH address: {}", e))?;

    // Extract script hash from payload
    let script_hash = match address.payload {
        Payload::ScriptHash(hash) => hash,
        _ => return Err(anyhow!("Expected P2SH payload")),
    };

    // Convert to CashAddr format (bitcoincash:p...) using shared encoder
    let encoded =
        crate::market::cashaddr::cashaddr_encode("bitcoincash", 0x08, script_hash.as_ref())?;
    Ok(encoded)
}

// Removed simplified P2SH CashAddr encoder; using shared pure Rust encoder

/// Generate DOGE P2SH multisig address (A... or 9... for mainnet)
/// Uses Dogecoin script-hash version byte 0x16 (22 decimal)
pub fn generate_doge_multisig_address(config: &MultisigConfig) -> Result<String> {
    let redeem_script = build_multisig_redeem_script(config)?;

    // Calculate script hash (SHA256 then RIPEMD160)
    let script_bytes = redeem_script.as_bytes();
    let sha_hash = Sha256::digest(script_bytes);
    let script_hash = ripemd::Ripemd160::digest(&sha_hash);

    // Encode with Dogecoin P2SH version byte
    encode_dogecoin_p2sh_address(&script_hash)
}

/// Encode Dogecoin P2SH address using base58check
/// Version byte: 0x16 (22 decimal) for mainnet, produces addresses starting with 'A' or '9'
fn encode_dogecoin_p2sh_address(script_hash: &[u8]) -> Result<String> {
    const DOGE_P2SH_VERSION: u8 = 0x16; // Mainnet P2SH version

    let mut payload = vec![DOGE_P2SH_VERSION];
    payload.extend_from_slice(script_hash);

    // Checksum (double SHA256, first 4 bytes)
    let hash1 = Sha256::digest(&payload);
    let hash2 = Sha256::digest(&hash1);
    let checksum = &hash2[..4];

    payload.extend_from_slice(checksum);

    Ok(bs58::encode(&payload).into_string())
}

/// Get or generate all miners multisig addresses
pub fn get_miners_multisig_addresses() -> Result<(String, String, String, MultisigConfig)> {
    let config = MultisigConfig::from_env()?;

    let btc_addr = generate_btc_multisig_address(&config)?;
    let bch_addr = generate_bch_multisig_address(&config)?;
    let doge_addr = generate_doge_multisig_address(&config)?;

    Ok((btc_addr, bch_addr, doge_addr, config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multisig_config_parsing() {
        std::env::set_var("VISION_MINERS_MULTISIG_M", "2");
        std::env::set_var(
            "VISION_MINERS_MULTISIG_PUBKEYS",
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5,\
             03774ae7f858a9411e5ef4246b70c65aac5649980be5c17891bbec17895da008cb",
        );

        let config = MultisigConfig::from_env().unwrap();
        assert_eq!(config.m, 2);
        assert_eq!(config.n(), 2);
    }

    #[test]
    fn test_btc_multisig_address_generation() {
        let config = MultisigConfig {
            m: 2,
            pubkeys: vec![
                "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5".to_string(),
                "03774ae7f858a9411e5ef4246b70c65aac5649980be5c17891bbec17895da008cb".to_string(),
            ],
        };

        let addr = generate_btc_multisig_address(&config).unwrap();
        assert!(addr.starts_with("3"), "BTC P2SH must start with 3");
    }

    #[test]
    fn test_doge_multisig_address_generation() {
        let config = MultisigConfig {
            m: 2,
            pubkeys: vec![
                "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5".to_string(),
                "03774ae7f858a9411e5ef4246b70c65aac5649980be5c17891bbec17895da008cb".to_string(),
            ],
        };

        let addr = generate_doge_multisig_address(&config).unwrap();
        assert!(
            addr.starts_with("A") || addr.starts_with("9"),
            "DOGE P2SH must start with A or 9"
        );
    }

    #[test]
    fn test_multisig_is_deterministic() {
        let config = MultisigConfig {
            m: 2,
            pubkeys: vec![
                "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5".to_string(),
                "03774ae7f858a9411e5ef4246b70c65aac5649980be5c17891bbec17895da008cb".to_string(),
            ],
        };

        let addr1 = generate_btc_multisig_address(&config).unwrap();
        let addr2 = generate_btc_multisig_address(&config).unwrap();

        assert_eq!(addr1, addr2, "Multisig address must be deterministic");
    }
}
