use vision_market::crypto::addr::{bch_cashaddr_to_script, scripthash_hex};

#[test]
fn bch_cashaddr_p2pkh_builds_script_and_scripthash() {
    // Canonical P2PKH example from BCH docs
    let a = "bitcoincash:qpm2qsznhks23z7629mms6s4cwef74vcwvy22gdx6a";
    let spk = bch_cashaddr_to_script(a).expect("cashaddr decode");
    assert_eq!(spk.len(), 25, "P2PKH script should be 25 bytes");
    let sh = scripthash_hex(&spk);
    assert_eq!(sh.len(), 64);
}

#[test]
fn bch_cashaddr_without_prefix_also_parses() {
    let a = "qpm2qsznhks23z7629mms6s4cwef74vcwvy22gdx6a";
    let spk = bch_cashaddr_to_script(a).expect("parse cashaddr w/o hrp prefix");
    assert_eq!(spk.len(), 25);
}
