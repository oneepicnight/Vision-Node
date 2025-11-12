fn main() {
    std::env::set_var("BECH32_PERMISSIVE", "1");
    let addrs = [
        "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080",
        "bc1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gd0p7z9x8p2l6q2h3vx9y",
    ];
    for a in &addrs {
        println!("probe addr={}", a);
        match vision_market::crypto::addr::btc_address_to_script(a) {
            Some(s) => println!("ok script len={}", s.len()),
            None => println!("failed to decode {}", a),
        }
        debug_manual(a);
    }
}

#[allow(dead_code)]
fn debug_manual(addr: &str) {
    println!("--- debug manual for {} ---", addr);
    match bech32::decode(addr) {
        Ok((hrp, data, var)) => println!(
            "decode ok hrp={} data_len={} var={:?}",
            hrp,
            data.len(),
            var
        ),
        Err(e) => println!("decode err {:?}", e),
    }
    if let Some(pos) = addr.rfind('1') {
        let data_part = &addr[pos + 1..];
        println!("data_part_len={}", data_part.len());
        const CHARSET: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
        let mut u5s: Vec<u8> = Vec::new();
        for ch in data_part.chars() {
            if let Some(i) = CHARSET.find(ch) {
                u5s.push(i as u8);
            } else {
                println!("bad char {}", ch);
            }
        }
        println!("u5s_len={}", u5s.len());
        if u5s.len() > 6 {
            let u5_nocheck = &u5s[..u5s.len() - 6];
            println!("u5_nocheck_len={}", u5_nocheck.len());
            if !u5_nocheck.is_empty() {
                println!(
                    "ver={}, prog_u5_len={}",
                    u5_nocheck[0],
                    u5_nocheck.len() - 1
                );
                let prog = &u5_nocheck[1..];
                println!("prog sample first 12: {:?}", &prog[..prog.len().min(12)]);
                match bech32::convert_bits(prog, 5, 8, true) {
                    Ok(bytes) => println!("convert_bits ok len={}", bytes.len()),
                    Err(e) => println!("convert_bits err {:?}", e),
                }
            }
        }
    }
}
