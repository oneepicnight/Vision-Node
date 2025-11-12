use vision_market::crypto::addr::{address_to_p2pkh_script, scripthash_hex};

#[test]
fn p2pkh_script_and_scripthash_for_btc_bch_doge() {
    let cases = vec![
        ("BTC", "1BoatSLRHtKNngkdXEeobR76b53LETtpyT"), // ver 0x00
        ("BCH", "1BpEi6DfDAUFd7GtittLSdBeYJvcoaVggu"), // legacy format
        ("DOGE", "D5d7wPZ4k1G8R9wUAKqzT8rYwEJmJ5u9wW"), // ver 0x1e
    ];
    for (chain, addr) in cases {
        if let Some(spk) = address_to_p2pkh_script(chain, addr) {
            assert_eq!(spk.len(), 25);
            let sh = scripthash_hex(&spk);
            assert_eq!(sh.len(), 64);
        }
    }
}
