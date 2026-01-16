fn main() {
    // Single-world build: FULL only.
    // Keep build output clean (no cargo:warning spam) while still exposing the
    // build variant to the crate.
    println!("cargo:rustc-env=VISION_BUILD_VARIANT=FULL");
    
    // Security: Capture build timestamp for handshake verification
    // This allows nodes to identify outdated builds and reject very old peers
    let build_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    println!("cargo:rustc-env=BUILD_TIME_UNIX={}", build_timestamp);
    
    // Git commit hash (if available)
    if let Ok(output) = std::process::Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
    {
        if output.status.success() {
            let commit = String::from_utf8_lossy(&output.stdout);
            println!("cargo:rustc-env=GIT_COMMIT={}", commit.trim());
        }
    }
    
    // Rust compiler version
    if let Ok(output) = std::process::Command::new("rustc")
        .arg("--version")
        .output()
    {
        if output.status.success() {
            let rustc_ver = String::from_utf8_lossy(&output.stdout);
            println!("cargo:rustc-env=RUSTC_VER={}", rustc_ver.trim());
        }
    }
}
