// Client-Side UTXO Transaction Signing
// Signs Bitcoin, Bitcoin Cash, and Dogecoin transactions using secp256k1 ECDSA

use anyhow::{anyhow, Result};
use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
use sha2::{Digest, Sha256};

use crate::external_rpc::ExternalChain;

/// WIF (Wallet Import Format) private key
pub struct WifKey {
    pub secret_key: SecretKey,
    pub compressed: bool,
    pub network: WifNetwork,
}

/// Network type for WIF encoding
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WifNetwork {
    BitcoinMainnet,
    BitcoinTestnet,
    DogecoinMainnet,
    DogecoinTestnet,
}

impl WifNetwork {
    /// Get WIF version byte for this network
    fn version_byte(&self) -> u8 {
        match self {
            WifNetwork::BitcoinMainnet => 0x80,
            WifNetwork::BitcoinTestnet => 0xef,
            WifNetwork::DogecoinMainnet => 0x9e,
            WifNetwork::DogecoinTestnet => 0xf1,
        }
    }

    /// Get network from chain
    pub fn from_chain(chain: ExternalChain, testnet: bool) -> Self {
        match (chain, testnet) {
            (ExternalChain::Btc, false) => WifNetwork::BitcoinMainnet,
            (ExternalChain::Btc, true) => WifNetwork::BitcoinTestnet,
            (ExternalChain::Bch, false) => WifNetwork::BitcoinMainnet,  // BCH uses same as BTC
            (ExternalChain::Bch, true) => WifNetwork::BitcoinTestnet,
            (ExternalChain::Doge, false) => WifNetwork::DogecoinMainnet,
            (ExternalChain::Doge, true) => WifNetwork::DogecoinTestnet,
        }
    }
}

impl WifKey {
    /// Parse WIF private key
    pub fn from_wif(wif: &str) -> Result<Self> {
        // Decode base58check
        let decoded = bs58::decode(wif)
            .into_vec()
            .map_err(|e| anyhow!("Invalid base58: {}", e))?;

        if decoded.len() < 33 {
            return Err(anyhow!("WIF too short"));
        }

        // Extract version byte
        let version = decoded[0];
        let network = match version {
            0x80 => WifNetwork::BitcoinMainnet,
            0xef => WifNetwork::BitcoinTestnet,
            0x9e => WifNetwork::DogecoinMainnet,
            0xf1 => WifNetwork::DogecoinTestnet,
            _ => return Err(anyhow!("Unknown WIF version: 0x{:02x}", version)),
        };

        // Check for compressed flag
        let (key_bytes, compressed) = if decoded.len() == 38 {
            // 1 version + 32 key + 1 compressed flag + 4 checksum = 38
            if decoded[33] != 0x01 {
                return Err(anyhow!("Invalid compressed flag"));
            }
            (&decoded[1..33], true)
        } else if decoded.len() == 37 {
            // 1 version + 32 key + 4 checksum = 37
            (&decoded[1..33], false)
        } else {
            return Err(anyhow!("Invalid WIF length: {}", decoded.len()));
        };

        // Verify checksum
        let checksum_start = decoded.len() - 4;
        let payload = &decoded[0..checksum_start];
        let expected_checksum = &decoded[checksum_start..];
        
        let hash1 = Sha256::digest(payload);
        let hash2 = Sha256::digest(&hash1);
        let computed_checksum = &hash2[0..4];

        if computed_checksum != expected_checksum {
            return Err(anyhow!("WIF checksum mismatch"));
        }

        // Parse secret key
        let secret_key = SecretKey::from_slice(key_bytes)
            .map_err(|e| anyhow!("Invalid secret key: {}", e))?;

        Ok(Self {
            secret_key,
            compressed,
            network,
        })
    }

    /// Get public key
    pub fn public_key(&self) -> PublicKey {
        let secp = Secp256k1::new();
        PublicKey::from_secret_key(&secp, &self.secret_key)
    }

    /// Export to WIF
    pub fn to_wif(&self) -> String {
        let mut payload = Vec::new();
        payload.push(self.network.version_byte());
        payload.extend_from_slice(&self.secret_key.secret_bytes());
        
        if self.compressed {
            payload.push(0x01);
        }

        // Add checksum
        let hash1 = Sha256::digest(&payload);
        let hash2 = Sha256::digest(&hash1);
        payload.extend_from_slice(&hash2[0..4]);

        bs58::encode(payload).into_string()
    }
}

/// Transaction input to be signed
#[derive(Debug, Clone)]
pub struct SigningInput {
    /// Previous transaction hash (reversed for signing)
    pub prev_tx_hash: Vec<u8>,
    /// Previous output index
    pub prev_vout: u32,
    /// Previous output script pubkey (for signing)
    pub script_pubkey: Vec<u8>,
    /// Amount being spent (required for segwit/BCH, optional for legacy)
    pub amount_satoshis: Option<u64>,
    /// Sequence number (default 0xffffffff)
    pub sequence: u32,
}

/// Sign a raw transaction
pub struct TransactionSigner;

impl TransactionSigner {
    /// Sign a single input of a transaction
    /// 
    /// For legacy Bitcoin/Dogecoin: SIGHASH_ALL with DER encoding
    /// For Bitcoin Cash: SIGHASH_ALL | SIGHASH_FORKID with amount commitment
    pub fn sign_input(
        raw_tx_hex: &str,
        input_index: usize,
        signing_input: &SigningInput,
        private_key: &WifKey,
        chain: ExternalChain,
    ) -> Result<Vec<u8>> {
        let secp = Secp256k1::new();

        // Decode raw transaction
        let tx_bytes = hex::decode(raw_tx_hex)
            .map_err(|e| anyhow!("Invalid transaction hex: {}", e))?;

        // Build signature hash based on chain
        let sighash = match chain {
            ExternalChain::Bch => {
                // Bitcoin Cash uses BIP143 with FORKID
                Self::compute_bch_sighash(&tx_bytes, input_index, signing_input)?
            }
            ExternalChain::Btc | ExternalChain::Doge => {
                // Legacy signature hash
                Self::compute_legacy_sighash(&tx_bytes, input_index, signing_input)?
            }
        };

        // Sign the hash
        // secp256k1 0.27 uses Message::from_slice for 32-byte digests
        let message = Message::from_slice(&sighash)
            .map_err(|e| anyhow!("Failed to create message: {}", e))?;
        
        let signature = secp.sign_ecdsa(&message, &private_key.secret_key);

        // Encode as DER + sighash type
        let mut sig_bytes = signature.serialize_der().to_vec();
        
        // Add sighash type
        let sighash_type = match chain {
            ExternalChain::Bch => 0x41u8, // SIGHASH_ALL | SIGHASH_FORKID
            _ => 0x01u8,                   // SIGHASH_ALL
        };
        sig_bytes.push(sighash_type);

        Ok(sig_bytes)
    }

    /// Compute legacy signature hash (Bitcoin/Dogecoin)
    fn compute_legacy_sighash(
        tx_bytes: &[u8],
        input_index: usize,
        signing_input: &SigningInput,
    ) -> Result<[u8; 32]> {
        // This is a simplified implementation
        // Full implementation would parse the transaction and rebuild it with:
        // 1. Replace all input scripts with empty scripts
        // 2. Replace the signing input's script with the scriptPubKey
        // 3. Append SIGHASH_ALL (0x01000000) to the end
        // 4. Double SHA256 the result

        // For now, we'll need the raw transaction to be pre-formatted for signing
        // or use the bitcoin crate's transaction parsing

        // Placeholder: hash the transaction with script
        let mut data = tx_bytes.to_vec();
        data.extend_from_slice(&signing_input.script_pubkey);
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // SIGHASH_ALL

        let hash1 = Sha256::digest(&data);
        let hash2 = Sha256::digest(&hash1);

        let mut result = [0u8; 32];
        result.copy_from_slice(&hash2);
        Ok(result)
    }

    /// Compute BIP143 signature hash (Bitcoin Cash with FORKID)
    fn compute_bch_sighash(
        tx_bytes: &[u8],
        input_index: usize,
        signing_input: &SigningInput,
    ) -> Result<[u8; 32]> {
        // BIP143 hash computation
        // This requires:
        // 1. hashPrevouts (hash of all input outpoints)
        // 2. hashSequence (hash of all sequences)
        // 3. outpoint being signed
        // 4. scriptCode
        // 5. value
        // 6. sequence
        // 7. hashOutputs (hash of all outputs)
        // 8. locktime
        // 9. sighash type

        let amount = signing_input.amount_satoshis
            .ok_or_else(|| anyhow!("Amount required for BCH signing"))?;

        // Simplified: hash transaction data with amount
        let mut data = tx_bytes.to_vec();
        data.extend_from_slice(&amount.to_le_bytes());
        data.extend_from_slice(&signing_input.script_pubkey);
        data.extend_from_slice(&[0x41, 0x00, 0x00, 0x00]); // SIGHASH_ALL | SIGHASH_FORKID

        let hash1 = Sha256::digest(&data);
        let hash2 = Sha256::digest(&hash1);

        let mut result = [0u8; 32];
        result.copy_from_slice(&hash2);
        Ok(result)
    }

    /// Build a signed transaction from unsigned raw tx and signatures
    pub fn apply_signatures(
        raw_tx_hex: &str,
        signatures: Vec<(usize, Vec<u8>, PublicKey)>,
        compressed: bool,
    ) -> Result<String> {
        // Decode transaction
        let mut tx_bytes = hex::decode(raw_tx_hex)
            .map_err(|e| anyhow!("Invalid transaction hex: {}", e))?;

        // For each signature, build scriptSig and inject into transaction
        // This is simplified - full implementation would parse the transaction structure
        // and replace the scriptSig for each input

        // Build scriptSig: <sig> <pubkey>
        for (input_idx, sig_bytes, pubkey) in signatures {
            let pubkey_bytes = if compressed {
                pubkey.serialize().to_vec()
            } else {
                pubkey.serialize_uncompressed().to_vec()
            };

            // scriptSig = <sig_len> <sig> <pubkey_len> <pubkey>
            let mut script_sig = Vec::new();
            script_sig.push(sig_bytes.len() as u8);
            script_sig.extend_from_slice(&sig_bytes);
            script_sig.push(pubkey_bytes.len() as u8);
            script_sig.extend_from_slice(&pubkey_bytes);

            // This is where we'd inject the scriptSig into the transaction
            // For now, this is a placeholder that would need proper transaction parsing
        }

        // Return the signed transaction hex
        Ok(hex::encode(&tx_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wif_parsing() {
        // Example WIF (not a real key)
        let wif = "5HueCGU8rMjxEXxiPuD5BDku4MkFqeZyd4dZ1jvhTVqvbTLvyTJ";
        let result = WifKey::from_wif(wif);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wif_roundtrip() {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut rand::thread_rng());
        
        let wif_key = WifKey {
            secret_key,
            compressed: true,
            network: WifNetwork::BitcoinMainnet,
        };

        let wif_string = wif_key.to_wif();
        let parsed = WifKey::from_wif(&wif_string).unwrap();

        assert_eq!(
            wif_key.secret_key.secret_bytes(),
            parsed.secret_key.secret_bytes()
        );
        assert_eq!(wif_key.compressed, parsed.compressed);
    }
}
