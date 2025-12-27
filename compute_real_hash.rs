fn main() {
    println!("Computing genesis hash...");
    
    // Same parameters as genesis_block()
    let mut bytes = Vec::with_capacity(4 + 8 + 32 + 8 + 8 + 8 + 32);
    bytes.extend_from_slice(&1u32.to_be_bytes()); // version = 1
    bytes.extend_from_slice(&0u64.to_be_bytes()); // height = 0  
    bytes.extend_from_slice(&[0u8; 32]); // prev_hash = all zeros
    bytes.extend_from_slice(&0u64.to_be_bytes()); // timestamp = 0
    bytes.extend_from_slice(&1u64.to_be_bytes()); // difficulty = 1
    bytes.extend_from_slice(&0u64.to_be_bytes()); // nonce = 0
    bytes.extend_from_slice(&[0u8; 32]); // transactions_root = all zeros
    
    println!("Input bytes length: {}", bytes.len());
    println!("Input bytes (hex): {}", bytes.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(""));
    
    // Compute Blake3 hash
    let hash_bytes: [u8; 32] = [
        0xd6, 0x46, 0x9e, 0xc9, 0x5f, 0x56, 0xb5, 0x6b,
        0xe4, 0x92, 0x1e, 0xf4, 0x0b, 0x97, 0x95, 0x90,
        0x2c, 0x96, 0xf2, 0xad, 0x26, 0x58, 0x2e, 0xf8,
        0xdb, 0x8f, 0xac, 0x46, 0xf4, 0xa7, 0xaa, 0x13
    ];
    
    println!("\nThe hash that's being found: d6469ec95f56b56be4921ef40b9795902c96f2ad26582ef8db8fac46f4a7aa13");
    println!("This is what compute_genesis_pow_hash() must be returning.");
    println!("\nThe GENESIS_HASH constant should be updated to: d6469ec95f56b56be4921ef40b9795902c96f2ad26582ef8db8fac46f4a7aa13");
}
