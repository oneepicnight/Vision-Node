#[test]
fn cursor_roundtrip() {
    let pairs = vec![
        (0_i64, "a"),
        (1732147200_i64, "id-001"),
        (-1_i64, "weird|pipe? fine"),
        (42_i64, "abc123XYZ"),
    ];
    for (ts, id) in pairs {
        let c = vision_market::market::cursor::encode_cursor(ts, id);
        let back = vision_market::market::cursor::decode_cursor(&c).expect("decode");
        assert_eq!(back.0, ts);
        assert_eq!(back.1, id);
    }
}
