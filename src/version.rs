use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
pub struct VersionInfo {
    pub name: &'static str,
    pub version: &'static str,
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
        version: env!("CARGO_PKG_VERSION"),
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
