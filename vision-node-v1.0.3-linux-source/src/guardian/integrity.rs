//! Guardian Binary Integrity Verification
//!
//! Ensures the Guardian binary hasn't been tampered with by verifying SHA-256 hash
//! against a manifest file. Optional strict mode can abort startup if verification fails.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::Path;

/// Guardian integrity manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardianIntegrityManifest {
    /// Version string (e.g., "v0.8.1-testnet")
    pub version: String,
    /// Expected SHA-256 hash of vision-node.exe
    pub expected_sha256: String,
}

/// Result of integrity check
#[derive(Debug, Clone)]
pub struct IntegrityCheckResult {
    pub passed: bool,
    pub version: Option<String>,
    pub expected_hash: Option<String>,
    pub actual_hash: Option<String>,
    pub error: Option<String>,
}

impl IntegrityCheckResult {
    pub fn success(version: String, expected: String, actual: String) -> Self {
        Self {
            passed: true,
            version: Some(version),
            expected_hash: Some(expected),
            actual_hash: Some(actual),
            error: None,
        }
    }

    pub fn failure(
        version: Option<String>,
        expected: Option<String>,
        actual: String,
        error: String,
    ) -> Self {
        Self {
            passed: false,
            version,
            expected_hash: expected,
            actual_hash: Some(actual),
            error: Some(error),
        }
    }

    pub fn no_manifest() -> Self {
        Self {
            passed: false,
            version: None,
            expected_hash: None,
            actual_hash: None,
            error: Some("Manifest file not found".to_string()),
        }
    }
}

/// Load the integrity manifest from guardian_integrity.json
pub fn load_manifest(manifest_path: &Path) -> Result<GuardianIntegrityManifest, String> {
    if !manifest_path.exists() {
        return Err(format!(
            "Manifest file not found: {}",
            manifest_path.display()
        ));
    }

    let content =
        fs::read_to_string(manifest_path).map_err(|e| format!("Failed to read manifest: {}", e))?;

    let manifest: GuardianIntegrityManifest = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse manifest JSON: {}", e))?;

    Ok(manifest)
}

/// Compute SHA-256 hash of a file
pub fn compute_file_hash(file_path: &Path) -> Result<String, String> {
    if !file_path.exists() {
        return Err(format!("File not found: {}", file_path.display()));
    }

    let content = fs::read(file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();

    Ok(format!("{:x}", result))
}

/// Check if strict integrity mode is enabled
pub fn is_strict_mode_enabled() -> bool {
    env::var("GUARDIAN_STRICT_INTEGRITY")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false)
}

/// Perform full integrity check
pub fn verify_guardian_integrity(binary_path: &Path, manifest_path: &Path) -> IntegrityCheckResult {
    // Load manifest
    let manifest = match load_manifest(manifest_path) {
        Ok(m) => m,
        Err(_e) => {
            eprintln!("⚠️  Guardian integrity manifest missing – skipping integrity check.");
            eprintln!("    Expected: {}", manifest_path.display());
            return IntegrityCheckResult::no_manifest();
        }
    };

    // Compute actual hash
    let actual_hash = match compute_file_hash(binary_path) {
        Ok(hash) => hash,
        Err(e) => {
            eprintln!("❌ Failed to compute Guardian binary hash: {}", e);
            return IntegrityCheckResult::failure(
                Some(manifest.version.clone()),
                Some(manifest.expected_sha256.clone()),
                String::new(),
                e,
            );
        }
    };

    // Compare hashes
    if actual_hash == manifest.expected_sha256 {
        eprintln!(
            "✅ Guardian binary integrity OK ({}, hash verified)",
            manifest.version
        );
        IntegrityCheckResult::success(manifest.version, manifest.expected_sha256, actual_hash)
    } else {
        eprintln!("❌ Guardian binary integrity FAILED!");
        eprintln!("   Expected: {}", manifest.expected_sha256);
        eprintln!("   Got:      {}", actual_hash);
        eprintln!("   Version:  {}", manifest.version);

        IntegrityCheckResult::failure(
            Some(manifest.version),
            Some(manifest.expected_sha256),
            actual_hash,
            "Hash mismatch".to_string(),
        )
    }
}

/// Perform integrity check and handle strict mode
pub fn check_guardian_integrity_or_abort() -> IntegrityCheckResult {
    // Determine paths
    let exe_path =
        env::current_exe().unwrap_or_else(|_| Path::new("vision-node.exe").to_path_buf());

    let manifest_path = exe_path
        .parent()
        .map(|p| p.join("guardian_integrity.json"))
        .unwrap_or_else(|| Path::new("guardian_integrity.json").to_path_buf());

    // Run check
    let result = verify_guardian_integrity(&exe_path, &manifest_path);

    // Handle strict mode
    if !result.passed && is_strict_mode_enabled() {
        eprintln!();
        eprintln!("🛑 GUARDIAN STRICT INTEGRITY MODE ENABLED");
        eprintln!("   The Guardian binary failed integrity verification.");
        eprintln!("   Startup aborted for security.");
        eprintln!();
        eprintln!("   To disable strict mode, set in .env:");
        eprintln!("   GUARDIAN_STRICT_INTEGRITY=false");
        eprintln!();
        std::process::exit(1);
    } else if !result.passed {
        eprintln!();
        eprintln!("⚠️  WARNING: Guardian is running with compromised integrity.");
        eprintln!("   Consider re-downloading the official Guardian package.");
        eprintln!("   To enforce strict checking, set in .env:");
        eprintln!("   GUARDIAN_STRICT_INTEGRITY=true");
        eprintln!();
    }

    result
}

/// Auto-update manifest structure (fetched from remote URL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoUpdateManifest {
    /// Latest version string
    pub latest_version: String,
    /// Download URL for latest Guardian build
    pub download_url: String,
    /// Expected SHA-256 of the download
    pub sha256: String,
}

/// Check if auto-update is enabled
pub fn is_auto_update_enabled() -> bool {
    env::var("GUARDIAN_AUTO_UPDATE_ENABLED")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false)
}

/// Get auto-update manifest URL from environment
pub fn get_update_manifest_url() -> Option<String> {
    env::var("GUARDIAN_UPDATE_MANIFEST_URL").ok()
}

/// Check for Guardian updates (stub - does not download)
pub async fn check_for_updates(
    current_version: &str,
) -> Result<Option<AutoUpdateManifest>, String> {
    if !is_auto_update_enabled() {
        return Ok(None);
    }

    let manifest_url = match get_update_manifest_url() {
        Some(url) => url,
        None => {
            eprintln!("⚠️  Auto-update enabled but GUARDIAN_UPDATE_MANIFEST_URL not set");
            return Ok(None);
        }
    };

    eprintln!("🔍 Checking for Guardian updates...");
    eprintln!("   Current version: {}", current_version);
    eprintln!("   Manifest URL: {}", manifest_url);

    // Fetch manifest
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&manifest_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch update manifest: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Update manifest returned status: {}",
            response.status()
        ));
    }

    let manifest: AutoUpdateManifest = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse update manifest: {}", e))?;

    // Compare versions
    if manifest.latest_version != current_version {
        eprintln!();
        eprintln!(
            "🆕 New Guardian version available: {}",
            manifest.latest_version
        );
        eprintln!("   Download: {}", manifest.download_url);
        eprintln!("   SHA-256: {}", manifest.sha256);
        eprintln!();
        eprintln!("⚠️  Auto-update stub: not downloading yet. Manual update required.");
        eprintln!("   Future versions may support automatic download and verification.");
        eprintln!();

        Ok(Some(manifest))
    } else {
        eprintln!("✅ Guardian is up to date ({})", current_version);
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_compute_hash() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_integrity.bin");

        // Create test file
        let mut file = fs::File::create(&test_file).unwrap();
        file.write_all(b"test content").unwrap();

        // Compute hash
        let hash = compute_file_hash(&test_file).unwrap();

        // Verify it's a valid SHA-256 (64 hex chars)
        assert_eq!(hash.len(), 64);

        // Clean up
        fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_manifest_parsing() {
        let json = r#"{
            "version": "v0.8.1-testnet",
            "expected_sha256": "abc123def456"
        }"#;

        let manifest: GuardianIntegrityManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.version, "v0.8.1-testnet");
        assert_eq!(manifest.expected_sha256, "abc123def456");
    }

    #[test]
    fn test_auto_update_manifest_parsing() {
        let json = r#"{
            "latest_version": "v0.8.2-testnet",
            "download_url": "https://example.com/guardian.zip",
            "sha256": "abc123def456"
        }"#;

        let manifest: AutoUpdateManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.latest_version, "v0.8.2-testnet");
        assert_eq!(manifest.download_url, "https://example.com/guardian.zip");
        assert_eq!(manifest.sha256, "abc123def456");
    }
}
