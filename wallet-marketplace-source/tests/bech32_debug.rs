// Diagnostic decode test for BIP-173 examples; use `bech32::decode` directly.

#[test]
fn decode_bip173_examples() {
    let addrs = [
        "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080",
        "bc1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gd0p7z9x8p2l6q2h3vx9y",
    ];
    for a in &addrs {
        match bech32::decode(a) {
            Ok((hrp, data, var)) => {
                println!(
                    "decode ok {} hrp={} data_len={} variant={:?}",
                    a,
                    hrp,
                    data.len(),
                    var
                );
            }
            Err(e) => {
                println!("decode err {} {:?}", a, e);
            }
        }
    }
}
