use std::{fs, io::Write, path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer};
use hex::FromHex;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

/// ========= Shared types (mirror the node) =========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tx {
    pub nonce: u64,
    pub sender_pubkey: String,
    pub access_list: Vec<String>,
    pub module: String,
    pub method: String,
    pub args: Vec<u8>,
    pub tip: u64,
    pub fee_limit: u64,
    pub sig: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CashTransferArgs {
    to: String,
    amount: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Payment {
    to: String,
    amount: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CashMultiMintArgs {
    mints: Vec<Payment>,
}

fn signable_tx_bytes(tx: &Tx) -> Vec<u8> {
    let mut tmp = tx.clone();
    tmp.sig.clear();
    serde_json::to_vec(&tmp).expect("serialize tx for signing")
}

/// ========= Key loading =========

#[derive(Debug, Clone, Deserialize)]
struct KeyFile {
    #[serde(default)]
    public_key: Option<String>,
    // 32-byte secret OR 64-byte keypair (hex)
    secret_key: String,
}

fn load_keypair(keys_path: &PathBuf) -> Keypair {
    let raw = fs::read_to_string(keys_path).expect("read keys file");
    let kf: KeyFile = serde_json::from_str(&raw).expect("keys.json should have secret_key hex");

    let sk_bytes = <Vec<u8>>::from_hex(kf.secret_key.trim()).expect("bad secret_key hex");
    let kp = match sk_bytes.len() {
        64 => Keypair::from_bytes(&sk_bytes).expect("bad 64-byte keypair"),
        32 => {
            let secret = SecretKey::from_bytes(&sk_bytes).expect("bad 32-byte secret");
            let public: PublicKey = (&secret).into();
            Keypair { secret, public }
        }
        _ => panic!("secret_key must decode to 32 or 64 bytes, got {}", sk_bytes.len()),
    };

    if let Some(pk_hex) = kf.public_key {
        let pk_hex = pk_hex.trim();
        if !pk_hex.is_empty() {
            match <Vec<u8>>::from_hex(pk_hex) {
                Ok(want) if want.len() == 32 => {
                    let got = kp.public.to_bytes();
                    if want[..] != got[..] {
                        eprintln!(
                            "⚠️  keys.json public_key doesn't match secret_key-derived public key.\n    Using derived: {}",
                            hex::encode(got)
                        );
                    }
                }
                Ok(other) => eprintln!("⚠️  keys.json public_key length {} != 32 bytes; ignoring.", other.len()),
                Err(_) => eprintln!("⚠️  keys.json public_key is not valid hex; ignoring."),
            }
        }
    }

    kp
}

/// ========= CLI =========

#[derive(Parser, Debug)]
#[command(name = "vision-cli", version, about = "Vision node helper CLI")]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate a new ed25519 keypair (writes JSON with public/secret hex)
    Keygen {
        /// Output path for the key file (JSON)
        #[arg(long)]
        out: PathBuf,

        /// Also print keys to stdout
        #[arg(long)]
        print: bool,
    },

    /// Sign a single cash::transfer
    SignTransfer {
        /// Sender public key (hex)
        #[arg(long)]
        from: String,

        /// Recipient account id/public key (e.g. "miner" or hex)
        #[arg(long)]
        to: String,

        /// Token amount to send
        #[arg(long)]
        amount: u128,

        /// Sender nonce
        #[arg(long)]
        nonce: u64,

        /// Path to keys.json (with public_key/secret_key)
        #[arg(long)]
        keys: PathBuf,

        /// Miner tip
        #[arg(long, default_value_t = 0)]
        tip: u64,

        /// Print only the raw JSON envelope (no tutorial text)
        #[arg(long = "raw-json", default_value_t = false)]
        raw_json: bool,

        /// Write output to a file (instead of stdout)
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Sign a GM-only cash::multi_mint (args = { "mints": [...] })
    SignMultiMint {
        /// Gamemaster public key (hex) — must match on-chain GM
        #[arg(long)]
        from: String,

        /// GM nonce
        #[arg(long)]
        nonce: u64,

        /// Path to keys.json (with public_key/secret_key)
        #[arg(long)]
        keys: PathBuf,

        /// Miner tip (not used by executor, but kept for parity)
        #[arg(long, default_value_t = 0)]
        tip: u64,

        /// CSV file "addr,amount" per line
        #[arg(long = "mints-csv")]
        mints_csv: Option<PathBuf>,

        /// Inline JSON array: [{ "to":"...", "amount":123 }, ...]
        #[arg(long = "mints-json")]
        mints_json: Option<String>,

        /// Print only the raw JSON envelope (no tutorial text)
        #[arg(long = "raw-json", default_value_t = false)]
        raw_json: bool,

        /// Write output to a file (instead of stdout)
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        Commands::Keygen { out, print } => {
            let mut csprng = OsRng {};
            let kp = Keypair::generate(&mut csprng);

            let public_hex = hex::encode(kp.public.to_bytes());
            let secret_hex = hex::encode(kp.secret.to_bytes());

            let obj = serde_json::json!({
                "public_key": public_hex,
                "secret_key": secret_hex
            });

            fs::write(&out, serde_json::to_vec_pretty(&obj).unwrap()).expect("write key file");
            if print {
                println!("{}", serde_json::to_string_pretty(&obj).unwrap());
            }
        }

        Commands::SignTransfer {
            from,
            to,
            amount,
            nonce,
            keys,
            tip,
            raw_json,
            out,
        } => {
            let kp = load_keypair(&keys);

            let args = serde_json::to_vec(&CashTransferArgs { to: to.clone(), amount }).unwrap();

            // Node requires access to acct:FROM and acct:TO for transfer
            let access_list = vec![format!("acct:{from}"), format!("acct:{to}")];

            let mut tx = Tx {
                nonce,
                sender_pubkey: from.clone(),
                access_list,
                module: "cash".into(),
                method: "transfer".into(),
                args,
                tip,
                fee_limit: 0,
                sig: String::new(),
            };

            let msg = signable_tx_bytes(&tx);
            let sig: Signature = kp.sign(&msg);
            tx.sig = hex::encode(sig.to_bytes());

            let env = serde_json::json!({ "tx": tx });
            output(env, raw_json, out);
        }

        Commands::SignMultiMint {
            from,
            nonce,
            keys,
            tip,
            mints_csv,
            mints_json,
            raw_json,
            out,
        } => {
            let kp = load_keypair(&keys);

            let mints: Vec<Payment> = if let Some(csv_path) = mints_csv {
                let mut v = Vec::new();
                let txt = fs::read_to_string(csv_path).expect("read mints csv");
                for (lineno, line) in txt.lines().enumerate() {
                    let t = line.trim();
                    if t.is_empty() { continue; }
                    let parts: Vec<&str> = t.split(',').map(|s| s.trim()).collect();
                    if parts.len() != 2 {
                        panic!("bad csv at line {}", lineno + 1);
                    }
                    let to = parts[0].to_string();
                    let amount = u128::from_str(parts[1]).expect("invalid amount");
                    v.push(Payment { to, amount });
                }
                v
            } else if let Some(js) = mints_json {
                serde_json::from_str::<Vec<Payment>>(&js).expect("bad mints json")
            } else {
                panic!("provide --mints-csv or --mints-json");
            };

            if mints.is_empty() {
                panic!("empty mints");
            }

            let args = serde_json::to_vec(&CashMultiMintArgs { mints: mints.clone() }).unwrap();

            // Node’s multi_mint requires access_list of each destination ONLY
            let access_list: Vec<String> = mints.iter().map(|p| format!("acct:{}", p.to)).collect();

            let mut tx = Tx {
                nonce,
                sender_pubkey: from.clone(),
                access_list,
                module: "cash".into(),
                method: "multi_mint".into(),
                args,
                tip,
                fee_limit: 0,
                sig: String::new(),
            };

            let msg = signable_tx_bytes(&tx);
            let sig: Signature = kp.sign(&msg);
            tx.sig = hex::encode(sig.to_bytes());

            let env = serde_json::json!({ "tx": tx });
            output(env, raw_json, out);
        }
    }
}

/// Write stdout or file, with optional tutorial text
fn output(envelope: serde_json::Value, raw_only: bool, out: Option<PathBuf>) {
    let raw = serde_json::to_string(&envelope).unwrap();

    let body = if raw_only {
        raw.clone()
    } else {
        format!(
            "POST this to your node (adjust host:port if needed):\n\
             curl -s http://127.0.0.1:7070/submit_tx -H 'content-type: application/json' -d '{raw}'\n"
        )
    };

    if let Some(path) = out {
        let mut f = fs::File::create(path).expect("open output file");
        f.write_all(body.as_bytes()).expect("write output");
    } else {
        println!("{body}");
    }
}
