use std::collections::HashMap;
use std::str::FromStr;

fn charset_rev() -> HashMap<char, u8> {
    let s = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    let mut m = HashMap::new();
    for (i, c) in s.chars().enumerate() {
        m.insert(c, i as u8);
    }
    m
}

fn hrp_expand(hrp: &str) -> Vec<u8> {
    let mut v = Vec::new();
    for b in hrp.bytes() {
        v.push(b >> 5);
    }
    v.push(0);
    for b in hrp.bytes() {
        v.push(b & 0x1f);
    }
    v
}

fn polymod(values: &[u8]) -> u32 {
    let generators: [u32; 5] = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];
    let mut chk: u32 = 1;
    for v in values {
        let b = (chk >> 25) as u8;
        chk = (chk & 0x1ffffff) << 5 ^ (*v as u32);
        for (i, g) in generators.iter().enumerate() {
            if ((b >> i) & 1) != 0 {
                chk ^= g;
            }
        }
    }
    chk
}

fn verify_checksum(hrp: &str, data: &[u8]) -> bool {
    let mut values = hrp_expand(hrp);
    values.extend_from_slice(data);
    let pm = polymod(&values);
    println!("DEBUG: hrp_expand_len={} data_len={} polymod={}", values.len(), data.len(), pm);
    pm == 1
}

fn decode_bech32_manual(s: &str) -> Result<(String, Vec<u8>), String> {
    // find separator
    if s.len() < 8 {
        return Err("too short".to_string());
    }
    let pos = s.rfind('1').ok_or("no separator")?;
    let hrp = &s[..pos];
    let data_part = &s[pos + 1..];
    if hrp.is_empty() || data_part.len() < 6 {
        return Err("invalid parts".to_string());
    }
    let rev = charset_rev();
    let mut data = Vec::new();
    for c in data_part.chars() {
        let cl = c.to_ascii_lowercase();
        match rev.get(&cl) {
            Some(v) => data.push(*v),
            None => return Err(format!("invalid char {}", c)),
        }
    }
    println!("DEBUG: hrp='{}' data_part='{}' mapped={:?}", hrp, data_part, data);
    if !verify_checksum(hrp, &data) {
        return Err("invalid checksum".to_string());
    }
    // strip the 6 checksum chars
    let payload = data[..data.len() - 6].to_vec();
    Ok((hrp.to_string(), payload))
}

fn main() {
    let addrs = [
        "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080",
        "bc1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gd0p7z9x8p2l6q2h3vx9y",
    ];
    for a in &addrs {
        println!("address: {}", a);
        println!("bytes: {:?}", a.as_bytes());
        match decode_bech32_manual(a) {
            Ok((hrp, data)) => println!("manual decode ok hrp={} data_len={}", hrp, data.len()),
            Err(e) => println!("manual decode err {}", e),
        }
        match bitcoin::Address::from_str(a) {
            Ok(addr) => println!("bitcoin::Address ok: {}", addr),
            Err(e) => println!("bitcoin::Address err: {:?}", e),
        }
        // (re-encoding experiment removed)
    }
}
