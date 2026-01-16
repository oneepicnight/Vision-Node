use serde::{Deserialize, Serialize};
use std::io::Write;
use std::net::TcpStream;

const MAGIC: &[u8] = b"VISION-P2"; // 9 bytes

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HandshakeWireV3 {
    pub protocol_version: u32,
    pub chain_id: String,
    pub genesis_hash: String,
    pub node_nonce: u64,
    pub chain_height: u64,
    #[serde(default)]
    pub tip_height: Option<u64>,
    pub node_version: u32,
    #[serde(default)]
    pub network_id: String,
    #[serde(default)]
    pub node_build: String,

    pub node_tag: String,
    pub admission_ticket: String,
    #[serde(default)]
    pub passport: Option<serde_json::Value>,

    pub vision_address: String,
    pub node_id: String,
    pub public_key: String,
    pub role: String,

    #[serde(default)]
    pub ebid: String,
    #[serde(default)]
    pub is_guardian: bool,
    #[serde(default)]
    pub is_guardian_candidate: bool,
    #[serde(default)]
    pub http_api_port: Option<u16>,

    #[serde(default)]
    pub advertised_ip: Option<String>,
    #[serde(default)]
    pub advertised_port: Option<u16>,

    #[serde(default)]
    pub bootstrap_checkpoint_height: u64,
    #[serde(default)]
    pub bootstrap_checkpoint_hash: String,
    #[serde(default)]
    pub bootstrap_prefix: String,

    /// Curated seed peers (public routable P2P endpoints, ip:7072)
    #[serde(default)]
    pub seed_peers: Vec<String>,
}

fn usage() -> ! {
    eprintln!(
        "Usage:\n  cargo run --release --bin handshake_probe -- --addr 127.0.0.1:17072 --mode wrong_drop\n  cargo run --release --bin handshake_probe -- --addr 127.0.0.1:17072 --mode v1\n\nModes:\n  wrong_drop   Sends v3 handshake with mismatched bootstrap_prefix\n  v1           Sends framed handshake header with version=1 (expects server to reject newer versions)"
    );
    std::process::exit(2);
}

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        usage();
    }

    let addr = arg_value(&args, "--addr").unwrap_or_else(|| "127.0.0.1:17072".to_string());
    let mode = arg_value(&args, "--mode").unwrap_or_else(|| "wrong_drop".to_string());

    let mut stream = TcpStream::connect(&addr)?;

    match mode.as_str() {
        "v1" => {
            stream.write_all(MAGIC)?;
            stream.write_all(&[1u8])?; // version byte
            stream.flush()?;
            Ok(())
        }
        "wrong_drop" => {
            let wire = HandshakeWireV3 {
                protocol_version: 2,
                chain_id: "00".repeat(32),
                genesis_hash: "11".repeat(32),
                node_nonce: 0x1234_5678_9abc_def0,
                chain_height: 0,
                tip_height: Some(0),
                node_version: 100,
                network_id: "mainnet".to_string(),
                node_build: "v1.0.0-probe".to_string(),

                node_tag: "PROBE".to_string(),
                admission_ticket: "".to_string(),
                passport: None,

                vision_address: "".to_string(),
                node_id: "probe".to_string(),
                public_key: "".to_string(),
                role: "constellation".to_string(),

                ebid: "probe".to_string(),
                is_guardian: false,
                is_guardian_candidate: false,
                http_api_port: Some(17070),

                advertised_ip: None,
                advertised_port: None,

                bootstrap_checkpoint_height: 0,
                bootstrap_checkpoint_hash: "".to_string(),
                bootstrap_prefix: "drop-WRONG".to_string(),

                seed_peers: Vec::new(),
            };

            let payload = bincode::serialize(&wire)?;
            if payload.len() > u16::MAX as usize {
                return Err("payload too large".into());
            }
            let len = (payload.len() as u16).to_be_bytes();

            stream.write_all(MAGIC)?;
            stream.write_all(&[3u8])?; // version byte
            stream.write_all(&len)?;
            stream.write_all(&payload)?;
            stream.flush()?;
            Ok(())
        }
        _ => usage(),
    }
}
