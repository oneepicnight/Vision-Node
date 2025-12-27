// CashAddr (Bitcoin Cash) Encoder/Decoder - Pure Rust
// Implements CashAddr per spec: https://github.com/Bitcoin-UAHF/spec/blob/master/cashaddr.md
// - Prefix: e.g., "bitcoincash"
// - Version (5-bit): P2PKH=0, P2SH=8; size code 0 for 160-bit payload
// - Payload: 160-bit (20 bytes) hash for P2PKH/P2SH

use anyhow::{anyhow, Result};

const CHARSET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
const GENERATOR: [u64; 5] = [
    0x98f2bc8e61,
    0x79b76d99e2,
    0xf33e5fb3c4,
    0xae2eabe2a8,
    0x1e4f43e470,
];

fn charset_index(ch: u8) -> Option<u8> {
    CHARSET.iter().position(|&c| c == ch).map(|i| i as u8)
}

fn hrp_expand(hrp: &str) -> Vec<u8> {
    let hrp = hrp.to_ascii_lowercase();
    let mut ret = Vec::with_capacity(hrp.len() * 2 + 1);
    for b in hrp.as_bytes() {
        ret.push(*b >> 5);
    }
    ret.push(0);
    for b in hrp.as_bytes() {
        ret.push(*b & 0x1f);
    }
    ret
}

fn polymod(values: &[u8]) -> u64 {
    let mut chk: u64 = 1;
    for v in values {
        let top = chk >> 35;
        chk = ((chk & 0x07ffffffff) << 5) ^ (*v as u64);
        for i in 0..5 {
            if ((top >> i) & 1) != 0 {
                chk ^= GENERATOR[i];
            }
        }
    }
    chk ^ 1
}

fn create_checksum(hrp: &str, data: &[u8]) -> [u8; 8] {
    let mut values = hrp_expand(hrp);
    values.extend_from_slice(data);
    values.extend_from_slice(&[0u8; 8]);
    let pm = polymod(&values);
    let mut checksum = [0u8; 8];
    let mut x = pm;
    for i in 0..8 {
        // 40 bits -> 8 groups of 5
        checksum[7 - i] = (x & 0x1f) as u8;
        x >>= 5;
    }
    checksum
}

fn convert_bits(data: &[u8], from: u32, to: u32, pad: bool) -> Result<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    let maxv: u32 = (1 << to) - 1;
    let mut ret: Vec<u8> = Vec::new();
    for value in data {
        let v = *value as u32;
        if (v >> from) != 0 {
            return Err(anyhow!(
                "convert_bits: invalid value {} for from={} bits",
                v,
                from
            ));
        }
        acc = (acc << from) | v;
        bits += from;
        while bits >= to {
            bits -= to;
            ret.push(((acc >> bits) & maxv) as u8);
        }
    }
    if pad {
        if bits > 0 {
            ret.push(((acc << (to - bits)) & maxv) as u8);
        }
    } else if bits >= from || ((acc << (to - bits)) & maxv) != 0 {
        return Err(anyhow!("convert_bits: non-zero padding"));
    }
    Ok(ret)
}

/// Encode CashAddr string
/// prefix: e.g., "bitcoincash"
/// version: 5-bit value (P2PKH=0, P2SH=8) including size code 0 for 160-bit
/// payload: 20-byte hash for 160-bit programs
pub fn cashaddr_encode(prefix: &str, version: u8, payload: &[u8]) -> Result<String> {
    if payload.len() != 20 {
        return Err(anyhow!(
            "CashAddr payload must be 20 bytes (160-bit), got {}",
            payload.len()
        ));
    }
    let mut data: Vec<u8> = Vec::with_capacity(1 + (payload.len() * 8 + 4) / 5);
    data.push(version & 0x1f);
    let five_bit = convert_bits(payload, 8, 5, true)?;
    data.extend_from_slice(&five_bit);
    let checksum = create_checksum(prefix, &data);
    let mut out = String::with_capacity(prefix.len() + 1 + data.len() + checksum.len());
    out.push_str(&prefix.to_ascii_lowercase());
    out.push(':');
    for d in data.iter().chain(checksum.iter()) {
        out.push(CHARSET[*d as usize] as char);
    }
    Ok(out)
}

/// Decode CashAddr string -> (prefix, version, payload)
pub fn cashaddr_decode(addr: &str) -> Result<(String, u8, Vec<u8>)> {
    let parts: Vec<&str> = addr.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow!("CashAddr must contain a ':', got '{}'", addr));
    }
    let hrp = parts[0].to_ascii_lowercase();
    let data_str = parts[1].to_ascii_lowercase();
    if data_str.is_empty() {
        return Err(anyhow!("CashAddr data part empty"));
    }
    let mut data: Vec<u8> = Vec::with_capacity(data_str.len());
    for b in data_str.bytes() {
        let Some(idx) = charset_index(b) else {
            return Err(anyhow!("Invalid CashAddr char '{}'", b as char));
        };
        data.push(idx);
    }
    if data.len() < 9 {
        return Err(anyhow!("CashAddr too short"));
    }
    // Verify checksum
    let (payload_part, checksum_part) = data.split_at(data.len() - 8);
    let mut values = hrp_expand(&hrp);
    values.extend_from_slice(payload_part);
    values.extend_from_slice(checksum_part);
    if polymod(&values) != 0 {
        return Err(anyhow!("Invalid CashAddr checksum"));
    }
    // Extract version and payload
    let version = payload_part[0];
    let payload5 = &payload_part[1..];
    let payload8 = convert_bits(payload5, 5, 8, false)?;
    Ok((hrp, version, payload8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cashaddr_roundtrip_known_p2pkh() {
        // Known valid BCH address used elsewhere in the repo
        let addr = "bitcoincash:qpm2qsznhks23z7629mms6s4cwef74vcwvy22gdx6a";
        let (prefix, version, payload) = cashaddr_decode(addr).unwrap();
        assert_eq!(prefix, "bitcoincash");
        assert_eq!(version & 0x08, 0x00); // P2PKH type bit clear
        assert_eq!(payload.len(), 20);
        let re = cashaddr_encode(&prefix, version, &payload).unwrap();
        assert_eq!(re, addr);
    }

    #[test]
    fn test_cashaddr_roundtrip_another_p2pkh() {
        let addr = "bitcoincash:qph2v4mkxjgkydg4w2l4r7nrw3xysxxcu659nzs28p";
        let (prefix, version, payload) = cashaddr_decode(addr).unwrap();
        assert_eq!(prefix, "bitcoincash");
        assert_eq!(version & 0x08, 0x00);
        assert_eq!(payload.len(), 20);
        let re = cashaddr_encode(&prefix, version, &payload).unwrap();
        assert_eq!(re, addr);
    }

    #[test]
    fn test_cashaddr_encode_p2sh_prefix() {
        // 20-byte dummy script hash; ensure resulting address starts with 'p' (P2SH)
        let payload = [0x11u8; 20];
        let addr = cashaddr_encode("bitcoincash", 0x08, &payload).unwrap();
        let part = addr.split(':').nth(1).unwrap();
        assert!(part.starts_with('p'));
        let (_hrp, ver, out) = cashaddr_decode(&addr).unwrap();
        assert_eq!(ver, 0x08);
        assert_eq!(out, payload);
    }
}
