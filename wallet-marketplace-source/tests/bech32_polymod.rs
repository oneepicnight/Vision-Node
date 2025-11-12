// Diagnostic helpers: hrp_expand and polymod per BIP-173
fn hrp_expand(hrp: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(hrp.len() * 2 + 1);
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
    let mut chk: u32 = 1;
    let generators: [u32; 5] = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];
    for v in values {
        let top: u32 = chk >> 25;
        chk = ((chk & 0x1ffffff) << 5) ^ (*v as u32);
        for (i, g) in generators.iter().enumerate() {
            if ((top >> i) & 1) != 0 {
                chk ^= g;
            }
        }
    }
    chk
}

const CHARSET: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";

#[test]
fn bech32_hrp_polymod_diagnostic() {
    let addrs = vec![
        "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080",
        "bc1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gd0p7z9x8p2l6q2h3vx9y",
    ];

    for addr in addrs {
        println!("--- diagnostic for {} ---", addr);
        let pos = addr.rfind('1').expect("invalid bech32 string: missing '1'");
        let hrp = &addr[..pos];
        let data_part = &addr[pos + 1..];
        println!("hrp='{}' data_part_len={}", hrp, data_part.len());

        // Convert data part chars to 5-bit values (u5)
        let mut data_u5: Vec<u8> = Vec::with_capacity(data_part.len());
        for c in data_part.chars() {
            match CHARSET.find(c) {
                Some(idx) => data_u5.push(idx as u8),
                None => panic!("invalid bech32 char: {}", c),
            }
        }

        let mut values = hrp_expand(hrp);
        values.extend_from_slice(&data_u5);

        let pm = polymod(&values);
        println!("polymod={} (expect 1 for valid bech32)", pm);

        match bech32::decode(addr) {
            Ok((d_hrp, _d_data, variant)) => {
                println!("bech32::decode ok hrp='{}' variant={:?}", d_hrp, variant)
            }
            Err(e) => println!("bech32::decode err {:?}", e),
        }
    }
}
