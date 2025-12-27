use blake3;
use hex;

fn main() {
    // Compute genesis hash exactly as in src/genesis.rs
    let mut bytes = Vec::with_capacity(4 + 8 + 32 + 8 + 8 + 8 + 32);
    bytes.extend_from_slice(&1u32.to_be_bytes()); // version
    bytes.extend_from_slice(&0u64.to_be_bytes()); // height
    bytes.extend_from_slice(&[0u8; 32]); // prev_hash
    bytes.extend_from_slice(&0u64.to_be_bytes()); // timestamp
    bytes.extend_from_slice(&1u64.to_be_bytes()); // difficulty
    bytes.extend_from_slice(&0u64.to_be_bytes()); // nonce
    bytes.extend_from_slice(&[0u8; 32]); // transactions_root
    
    let hash = blake3::hash(&bytes);
    let hash_hex = hex::encode(hash.as_bytes());
    
    println!("Computed genesis hash: {}", hash_hex);
    println!("Expected canonical:    af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262");
    
    if hash_hex == "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262" {
        println!("✅ MATCH - compute function is correct!");
    } else {
        println!("❌ MISMATCH - compute function produces wrong hash!");
    }
}
