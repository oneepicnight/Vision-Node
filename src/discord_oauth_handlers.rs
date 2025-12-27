// Discord OAuth wrapper handlers for main.rs
// These handlers work without axum State by using the global CHAIN database

use axum::{extract::Query, http::StatusCode, response::Redirect, Json};
use std::sync::Arc;

use crate::api::discord_oauth::{
    discord_callback as inner_callback, discord_login as inner_login,
    discord_status as inner_status, CallbackQuery, DiscordOAuthState, LoginQuery, StatusQuery,
};
use crate::CHAIN;
use sha2::Digest;

/// Default Discord OAuth credentials for public release
/// Can be overridden with environment variables if needed
const DEFAULT_DISCORD_CLIENT_ID: &str = "1442594705748529335";
const DEFAULT_DISCORD_CLIENT_SECRET: &str = "yT2lVs_9x9I8gccZJ1nacTkL1bSEUn39";
const DEFAULT_DISCORD_REDIRECT_URI: &str = "http://127.0.0.1:7070/api/discord/callback";

/// Lazy-initialized Discord OAuth state
fn get_discord_oauth_state() -> Result<DiscordOAuthState, String> {
    // Use defaults unless environment variables override them
    let client_id = std::env::var("DISCORD_CLIENT_ID")
        .unwrap_or_else(|_| DEFAULT_DISCORD_CLIENT_ID.to_string());

    let client_secret = std::env::var("DISCORD_CLIENT_SECRET")
        .unwrap_or_else(|_| DEFAULT_DISCORD_CLIENT_SECRET.to_string());

    let redirect_uri = std::env::var("DISCORD_REDIRECT_URI")
        .unwrap_or_else(|_| DEFAULT_DISCORD_REDIRECT_URI.to_string());

    // Use a consistent HMAC key derived from client secret
    let hmac_key = sha2::Sha256::digest(client_secret.as_bytes()).to_vec();

    // Get database from CHAIN
    let g = CHAIN.lock();
    let _db = g.db.clone();
    drop(g);

    // Open a shared connection for Discord links
    let conn = rusqlite::Connection::open("vision_discord_links.db")
        .map_err(|e| format!("Failed to open Discord links database: {}", e))?;

    // Initialize schema if needed
    crate::api::init_discord_links_db(&conn)
        .map_err(|e| format!("Failed to initialize Discord links schema: {}", e))?;

    Ok(DiscordOAuthState {
        db: Arc::new(std::sync::Mutex::new(conn)),
        client_id,
        client_secret,
        redirect_uri,
        hmac_key,
    })
}

/// GET /api/discord/login?wallet_address=...
pub async fn discord_oauth_login(
    Query(query): Query<LoginQuery>,
) -> Result<Json<crate::api::discord_oauth::LoginResponse>, (StatusCode, String)> {
    let state = get_discord_oauth_state().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Discord OAuth not configured: {}", e),
        )
    })?;

    inner_login(axum::extract::State(state), Query(query)).await
}

/// GET /api/discord/callback?code=...&state=...
pub async fn discord_oauth_callback(
    Query(query): Query<CallbackQuery>,
) -> Result<Redirect, (StatusCode, String)> {
    let state = get_discord_oauth_state().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Discord OAuth not configured: {}", e),
        )
    })?;

    inner_callback(axum::extract::State(state), Query(query)).await
}

/// GET /api/discord/status?wallet_address=...
pub async fn discord_oauth_status(
    Query(query): Query<StatusQuery>,
) -> Result<Json<crate::api::discord_oauth::StatusResponse>, (StatusCode, String)> {
    let state = get_discord_oauth_state().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Discord OAuth not configured: {}", e),
        )
    })?;

    inner_status(axum::extract::State(state), Query(query)).await
}
