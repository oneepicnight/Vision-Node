use base64::{engine::general_purpose, Engine as _};

pub fn encode_cursor(ts: i64, id: &str) -> String {
    general_purpose::STANDARD_NO_PAD.encode(format!("{}|{}", ts, id))
}

pub fn decode_cursor(s: &str) -> Option<(i64, String)> {
    let raw = general_purpose::STANDARD_NO_PAD.decode(s).ok()?;
    let text = String::from_utf8(raw).ok()?;
    let mut it = text.splitn(2, '|');
    let ts = it.next()?.parse::<i64>().ok()?;
    let id = it.next()?.to_string();
    Some((ts, id))
}
