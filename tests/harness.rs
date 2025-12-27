use std::process::Stdio;
use std::time::Duration;
use tempfile::TempDir;
use tokio::process::Command as TokioCommand;

use tokio::io::AsyncReadExt;

pub async fn spawn_node_with_opts(
    port: u16,
    data_dir: &str,
    mempool_max: Option<usize>,
    rate_submit_rps: Option<u64>,
    disable_p2p: bool,
    disable_miner: bool,
) -> (String, tokio::process::Child) {
    let bin_path = if cfg!(windows) {
        "target/debug/vision-node.exe"
    } else {
        "target/debug/vision-node"
    };
    if !std::path::Path::new(bin_path).exists() {
        panic!("Binary not built: {}", bin_path);
    }
    let mut cmd = TokioCommand::new(bin_path);

    cmd.env("VISION_PORT", port.to_string())
        .env("VISION_DATA_DIR", data_dir)
        .env("VISION_DEV", "1")
        .env("VISION_DEV_TOKEN", "devtest-token")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if let Some(m) = mempool_max {
        cmd.env("VISION_MEMPOOL_MAX", m.to_string());
    }
    if let Some(rps) = rate_submit_rps {
        cmd.env("VISION_RATE_SUBMIT_TX_RPS", rps.to_string());
    }
    if disable_p2p {
        cmd.env("VISION_DISABLE_P2P", "1");
    }
    if disable_miner {
        cmd.env("VISION_MINER_DISABLED", "1");
    }

    let mut child = cmd.spawn().expect("failed to spawn node process");
    let client = reqwest::Client::new();
    let base = format!("http://127.0.0.1:{}", port);
    let mut ready = false;
    for _ in 0..80 {
        if let Ok(resp) = client.get(&format!("{}/health", &base)).send().await {
            if resp.status().is_success() {
                ready = true;
                break;
            }
        }
        if let Ok(Some(status)) = child.try_wait() {
            // Try to read any stderr output
            if let Some(mut s) = child.stderr.take() {
                let mut buf = vec![];
                let _ = s.read_to_end(&mut buf).await;
                let out = String::from_utf8_lossy(&buf);
                panic!(
                    "node process exited early with status: {:?} stderr: {}",
                    status, out
                );
            }
            panic!("node process exited early with status: {:?}", status);
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    if !ready {
        let _ = child.kill().await;
        panic!("node did not start in time");
    }
    (base, child)
}

#[allow(dead_code)]
pub fn create_temp_data_dir() -> String {
    let tmp = TempDir::new().expect("tmpdir");
    tmp.path().to_string_lossy().to_string()
}
