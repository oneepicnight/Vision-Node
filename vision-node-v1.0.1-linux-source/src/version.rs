// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Vision Contributors

use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

// ============================
// SINGLE SOURCE OF TRUTH
// ============================

/// Software version (mainnet v1.0.0)
pub const VISION_VERSION: &str = "v1.0.0";

/// Network identifier (mainnet)
pub const VISION_NETWORK: &str = "mainnet";

// ============================

#[derive(Serialize)]
pub struct VersionInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub network: &'static str,
    pub git_commit: &'static str,
    pub build_time_unix: &'static str,
    pub rustc: &'static str,
    pub ts: u64,
}

async fn get_version() -> Json<VersionInfo> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Compile-time envs set by build.rs
    let git_commit = option_env!("GIT_COMMIT").unwrap_or("unknown");
    let rustc = option_env!("RUSTC_VER").unwrap_or("unknown");
    let build_time_unix = option_env!("BUILD_TIME_UNIX").unwrap_or("0");

    let info = VersionInfo {
        name: env!("CARGO_PKG_NAME"),
        version: VISION_VERSION,
        network: VISION_NETWORK,
        git_commit,
        build_time_unix,
        rustc,
        ts: now,
    };
    Json(info)
}

pub fn router() -> Router {
    Router::new().route("/version", get(get_version))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CI guard: Fail build if version contains testnet indicators
    #[test]
    fn test_no_testnet_in_version() {
        assert!(
            !VISION_VERSION.to_lowercase().contains("testnet"),
            "Version string must not contain 'testnet' for mainnet build"
        );
        assert!(
            !VISION_VERSION.to_lowercase().contains("test"),
            "Version string must not contain 'test' for mainnet build"
        );
        assert!(
            !VISION_VERSION.starts_with("v0."),
            "Version must not be v0.x for mainnet (use v1.0.0+)"
        );
    }

    /// CI guard: Fail build if network identifier is not mainnet
    #[test]
    fn test_network_is_mainnet() {
        assert_eq!(
            VISION_NETWORK, "mainnet",
            "Network identifier must be 'mainnet' for production builds"
        );
        assert!(
            !VISION_NETWORK.to_lowercase().contains("test"),
            "Network identifier must not contain 'test' for mainnet"
        );
    }

    /// CI guard: Verify no stray version prefixes
    #[test]
    fn test_version_format() {
        assert!(
            VISION_VERSION.starts_with("v"),
            "Version should start with 'v' (e.g., v1.0.0)"
        );
        assert!(
            VISION_VERSION.len() >= 5,
            "Version should be at least v1.0.0 format"
        );
    }
}
