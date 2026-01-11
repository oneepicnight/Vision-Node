use std::{process::Command, thread, time::Duration};

fn find_node_exe() -> std::path::PathBuf {
    let exe = std::env::current_exe().expect("current_exe failed");
    let dir = exe.parent().expect("no parent dir");

    #[cfg(windows)]
    let node_name = "vision-node.exe";
    #[cfg(not(windows))]
    let node_name = "vision-node";

    dir.join(node_name)
}

fn main() -> std::io::Result<()> {
    let node_exe = find_node_exe();
    println!("Vision launcher starting, node at: {}", node_exe.display());

    if !node_exe.exists() {
        eprintln!(
            "ERROR: Could not find vision-node executable at: {}",
            node_exe.display()
        );
        eprintln!("Make sure vision-node.exe is in the same directory as vision-launcher.exe");
        thread::sleep(Duration::from_secs(5));
        std::process::exit(1);
    }

    loop {
        println!("Launching vision-node...");

        match Command::new(&node_exe).spawn() {
            Ok(mut child) => {
                match child.wait() {
                    Ok(status) => {
                        println!("vision-node exited with status: {}", status);

                        // If node exited with non-zero status, wait a bit longer
                        // to avoid rapid restart loops on persistent errors
                        if !status.success() {
                            println!("Node exited with error, waiting 5 seconds before restart...");
                            thread::sleep(Duration::from_secs(5));
                        } else {
                            println!("Node exited cleanly, waiting 2 seconds before restart...");
                            thread::sleep(Duration::from_secs(2));
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to wait for node process: {}", e);
                        thread::sleep(Duration::from_secs(2));
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to spawn vision-node: {}", e);
                eprintln!("Waiting 5 seconds before retry...");
                thread::sleep(Duration::from_secs(5));
            }
        }
    }
}
