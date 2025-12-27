fn main() {
    let mut bytes = Vec::with_capacity(4 + 8 + 32 + 8 + 8 + 8 + 32);
    bytes.extend_from_slice(&1u32.to_be_bytes());
    bytes.extend_from_slice(&0u64.to_be_bytes());
    bytes.extend_from_slice(&[0u8; 32]);
    bytes.extend_from_slice(&0u64.to_be_bytes());
    bytes.extend_from_slice(&1u64.to_be_bytes());
    bytes.extend_from_slice(&0u64.to_be_bytes());
    bytes.extend_from_slice(&[0u8; 32]);
    println!("Bytes length: {}", bytes.len());
    println!("Bytes (hex): {}", bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>());
}
