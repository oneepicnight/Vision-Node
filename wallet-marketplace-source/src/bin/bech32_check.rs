use bech32::FromBase32;

fn main() {
    let addrs_bytes: [&[u8]; 2] = [
        &[
            98, 99, 49, 113, 119, 53, 48, 56, 100, 54, 113, 101, 106, 120, 116, 100, 103, 52, 121,
            53, 114, 51, 122, 97, 114, 118, 97, 114, 121, 48, 99, 53, 120, 119, 55, 107, 121, 103,
            116, 48, 56, 48,
        ],
        &[
            98, 99, 49, 113, 114, 112, 51, 51, 103, 48, 113, 53, 99, 53, 116, 120, 115, 112, 57,
            97, 114, 121, 115, 114, 120, 52, 107, 54, 122, 100, 107, 102, 115, 52, 110, 99, 101,
            52, 120, 106, 48, 103, 100, 48, 112, 55, 122, 57, 120, 56, 112, 50, 108, 54, 113, 50,
            104, 51, 118, 120, 57, 121,
        ],
    ];
    for bytes in &addrs_bytes {
        let a = String::from_utf8(bytes.to_vec()).expect("ascii");
        println!("address: {}", a);
        const CHARSET_LOCAL: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
        println!("CHARSET_LOCAL='{}'", CHARSET_LOCAL);
        println!("bytes (len={}): {:?}", a.len(), a.as_bytes());
        match bech32::decode(&a) {
            Ok((hrp, data, var)) => {
                println!(
                    "decode ok hrp={} data_len={} variant={:?}",
                    hrp,
                    data.len(),
                    var
                );
                let u5s: Vec<u8> = data.iter().map(|u| u.to_u8()).collect();
                println!("u5 sample: {:?}", &u5s[..u5s.len().min(12)]);
                match Vec::<u8>::from_base32(&data) {
                    Ok(bytes) => println!("from_base32 len={}", bytes.len()),
                    Err(e) => println!("from_base32 err {:?}", e),
                }
            }
            Err(e) => println!("decode err {:?}", e),
        }
        // try uppercase variant
        let au = a.to_uppercase();
        match bech32::decode(&au) {
            Ok((hrp, data, var)) => println!(
                "UPPER decode ok hrp={} data_len={} var={:?}",
                hrp,
                data.len(),
                var
            ),
            Err(e) => println!("UPPER decode err {:?}", e),
        }
        // try bitcoin crate parse
        use std::str::FromStr as _;
        match bitcoin::Address::from_str(&a) {
            Ok(addr) => println!("bitcoin parse ok payload={:?}", addr.payload),
            Err(e) => println!("bitcoin parse err {:?}", e),
        }
        // Manual hrp_expand + polymod
        fn hrp_expand(hrp: &str) -> Vec<u32> {
            let mut v = Vec::with_capacity(hrp.len() * 2 + 1);
            for b in hrp.bytes() {
                v.push((b >> 5) as u32);
            }
            v.push(0);
            for b in hrp.bytes() {
                v.push((b & 0x1f) as u32);
            }
            v
        }
        fn polymod(values: &[u32]) -> u32 {
            let mut chk: u32 = 1;
            const GEN: [u32; 5] = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];
            for v in values {
                let top = chk >> 25;
                chk = ((chk & 0x1ffffff) << 5) ^ (*v);
                for (i, g) in GEN.iter().enumerate() {
                    if ((top >> i) & 1) != 0 {
                        chk ^= *g;
                    }
                }
            }
            chk
        }
        if let Some(pos) = a.rfind('1') {
            let hrp = &a[..pos];
            let data_part = &a[pos + 1..];
            print!("hrp='{}' data_len={} ", hrp, data_part.len());
            const CHARSET: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
            let mut data_vals: Vec<u32> = Vec::with_capacity(data_part.len());
            for ch in data_part.chars() {
                match CHARSET.find(ch) {
                    Some(i) => data_vals.push(i as u32),
                    None => {
                        println!("char not in charset: {}", ch);
                    }
                }
            }
            let mut values = hrp_expand(hrp);
            values.extend(data_vals.iter().cloned());
            let pm = polymod(&values);
            println!("polymod={} values_len={}", pm, values.len());
        }
    }
}
