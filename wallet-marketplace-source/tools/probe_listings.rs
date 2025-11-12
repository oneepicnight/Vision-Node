use reqwest::blocking::Client;
use std::time::Duration;
fn main() {
    let client = Client::builder().timeout(Duration::from_secs(5)).build().unwrap();
    let url = "http://127.0.0.1:8080/market/land/listings";
    match client.get(url).send() {
        Ok(r) => {
            let status = r.status();
            match r.text() {
                Ok(t) => println!("fetched {} -> status={} body_len={}", url, status, t.len()),
                Err(e) => println!("fetched {} -> status={} but failed read text: {}", url, status, e),
            }
        }
        Err(e) => println!("GET {} failed: {}", url, e),
    }
}
