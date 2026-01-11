// Discord OAuth2 Integration for linking wallet addresses to Discord accounts
//
// This module provides endpoints for:
// 1. Initiating Discord OAuth2 flow
// 2. Handling OAuth2 callback
// 3. Checking Discord link status
// 4. Storing wallet ↔ Discord mappings

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Redirect,
    Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

type HmacSha256 = Hmac<Sha256>;

// ═══════════════════════════════════════════════════════════════════════════
//  GUARDIAN FEATURE-GATING
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(feature = "guardian")]
use crate::guardian::GuardianEvent;

#[cfg(not(feature = "guardian"))]
#[derive(Clone, Debug)]
pub struct GuardianEvent;

#[cfg(feature = "guardian")]
async fn emit_guardian_event(event: GuardianEvent) {
    // Call into the real guardian module when it exists
    let _ = crate::guardian::send_guardian_event(event).await;
}

#[cfg(not(feature = "guardian"))]
async fn emit_guardian_event(_event: GuardianEvent) {
    // No-op when the `guardian` feature is not enabled.
}

// ═══════════════════════════════════════════════════════════════════════════
//  DATABASE SCHEMA
// ═══════════════════════════════════════════════════════════════════════════

pub fn init_discord_links_db(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS discord_links (
            wallet_address TEXT PRIMARY KEY,
            discord_user_id TEXT NOT NULL,
            discord_username TEXT NOT NULL,
            linked_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_discord_user_id ON discord_links(discord_user_id)",
        [],
    )?;

    info!("[DISCORD OAUTH] Database initialized");
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  REQUEST / RESPONSE TYPES
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    pub wallet_address: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    pub wallet_address: String,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub linked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discord_user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discord_username: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StateToken {
    wallet_address: String,
    timestamp: i64,
}

#[derive(Debug, Deserialize)]
struct DiscordTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    discriminator: String,
}

// ═══════════════════════════════════════════════════════════════════════════
//  SHARED STATE
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct DiscordOAuthState {
    pub db: Arc<Mutex<Connection>>,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub hmac_key: Vec<u8>,
}

// ═══════════════════════════════════════════════════════════════════════════
//  ENDPOINT HANDLERS
// ═══════════════════════════════════════════════════════════════════════════

/// GET /api/discord/login?wallet_address=...
///
/// Initiates Discord OAuth2 flow by generating authorization URL
pub async fn discord_login(
    State(state): State<DiscordOAuthState>,
    Query(query): Query<LoginQuery>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    let wallet_address = query.wallet_address.trim();

    // Validate wallet address
    if wallet_address.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "wallet_address is required".to_string(),
        ));
    }

    if !wallet_address.starts_with("vision1") {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid wallet address format".to_string(),
        ));
    }

    // Create signed state token
    let state_token = StateToken {
        wallet_address: wallet_address.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    };

    let state_json = serde_json::to_string(&state_token)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let state_signed = sign_state(&state_json, &state.hmac_key);

    // Build Discord OAuth2 URL
    let auth_url = format!(
        "https://discord.com/api/oauth2/authorize?response_type=code&client_id={}&scope=identify&state={}&redirect_uri={}",
        state.client_id,
        urlencoding::encode(&state_signed),
        urlencoding::encode(&state.redirect_uri)
    );

    info!(
        "[DISCORD OAUTH] Login initiated for wallet: {}",
        wallet_address
    );

    Ok(Json(LoginResponse { url: auth_url }))
}

/// GET /api/discord/callback?code=...&state=...
///
/// Handles Discord OAuth2 callback, exchanges code for token, fetches user info, stores mapping
pub async fn discord_callback(
    State(state): State<DiscordOAuthState>,
    Query(query): Query<CallbackQuery>,
) -> Result<Redirect, (StatusCode, String)> {
    // Verify and decode state
    let state_json = verify_state(&query.state, &state.hmac_key)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid state: {}", e)))?;

    let state_token: StateToken = serde_json::from_str(&state_json).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid state format: {}", e),
        )
    })?;

    // Check state timestamp (valid for 10 minutes)
    let now = chrono::Utc::now().timestamp();
    if now - state_token.timestamp > 600 {
        return Err((StatusCode::BAD_REQUEST, "State expired".to_string()));
    }

    let wallet_address = state_token.wallet_address;

    // Exchange code for access token
    let client = reqwest::Client::new();

    let token_response = client
        .post("https://discord.com/api/oauth2/token")
        .form(&[
            ("client_id", state.client_id.as_str()),
            ("client_secret", state.client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", &query.code),
            ("redirect_uri", &state.redirect_uri),
        ])
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Token exchange failed: {}", e),
            )
        })?;

    if !token_response.status().is_success() {
        let error_text = token_response.text().await.unwrap_or_default();
        error!("[DISCORD OAUTH] Token exchange error: {}", error_text);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to get access token".to_string(),
        ));
    }

    let token_data: DiscordTokenResponse = token_response.json().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to parse token: {}", e),
        )
    })?;

    // Fetch Discord user info
    let user_response = client
        .get("https://discord.com/api/users/@me")
        .bearer_auth(&token_data.access_token)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch user: {}", e),
            )
        })?;

    if !user_response.status().is_success() {
        let error_text = user_response.text().await.unwrap_or_default();
        error!("[DISCORD OAUTH] User fetch error: {}", error_text);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to get user info".to_string(),
        ));
    }

    let discord_user: DiscordUser = user_response.json().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to parse user: {}", e),
        )
    })?;

    let discord_username = format!("{}#{}", discord_user.username, discord_user.discriminator);

    // Store mapping in database
    let db = state.db.lock().unwrap();
    db.execute(
        "INSERT OR REPLACE INTO discord_links (wallet_address, discord_user_id, discord_username, linked_at) VALUES (?1, ?2, ?3, ?4)",
        params![
            &wallet_address,
            &discord_user.id,
            &discord_username,
            chrono::Utc::now().timestamp(),
        ],
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    drop(db);

    info!(
        "[DISCORD OAUTH] Linked wallet {} to Discord user {} ({})",
        wallet_address, discord_username, discord_user.id
    );

    // Clone values for Guardian notification (non-blocking)
    let wallet_addr_clone = wallet_address.clone();
    let discord_id_clone = discord_user.id.clone();
    let discord_name_clone = discord_username.clone();
    tokio::spawn(async move {
        if let Err(e) =
            notify_guardian_link(&wallet_addr_clone, &discord_id_clone, &discord_name_clone).await
        {
            warn!("[DISCORD OAUTH] Failed to notify Guardian: {}", e);
        }
    });

    // Redirect back to wallet
    let redirect_url = format!(
        "/app/linked?wallet_address={}",
        urlencoding::encode(&wallet_address)
    );
    Ok(Redirect::to(&redirect_url))
}

/// GET /api/discord/status?wallet_address=...
///
/// Returns Discord link status for a wallet address
pub async fn discord_status(
    State(state): State<DiscordOAuthState>,
    Query(query): Query<StatusQuery>,
) -> Result<Json<StatusResponse>, (StatusCode, String)> {
    let wallet_address = query.wallet_address.trim();

    if wallet_address.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "wallet_address is required".to_string(),
        ));
    }

    let db = state.db.lock().unwrap();

    let result = db.query_row(
        "SELECT discord_user_id, discord_username FROM discord_links WHERE wallet_address = ?1",
        params![wallet_address],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    );

    drop(db);

    match result {
        Ok((discord_user_id, discord_username)) => Ok(Json(StatusResponse {
            linked: true,
            discord_user_id: Some(discord_user_id),
            discord_username: Some(discord_username),
        })),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(Json(StatusResponse {
            linked: false,
            discord_user_id: None,
            discord_username: None,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

fn sign_state(data: &str, key: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key");
    mac.update(data.as_bytes());
    let signature = mac.finalize().into_bytes();

    let mut combined = Vec::new();
    combined.extend_from_slice(data.as_bytes());
    combined.push(b'.');
    combined.extend_from_slice(&signature);

    URL_SAFE_NO_PAD.encode(&combined)
}

fn verify_state(signed: &str, key: &[u8]) -> Result<String, String> {
    let decoded = URL_SAFE_NO_PAD
        .decode(signed)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    let dot_pos = decoded
        .iter()
        .rposition(|&b| b == b'.')
        .ok_or("Invalid state format")?;

    let data = &decoded[..dot_pos];
    let signature = &decoded[dot_pos + 1..];

    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key");
    mac.update(data);

    mac.verify_slice(signature)
        .map_err(|_| "Signature verification failed")?;

    String::from_utf8(data.to_vec()).map_err(|e| format!("UTF-8 decode error: {}", e))
}

async fn notify_guardian_link(
    wallet_address: &str,
    discord_user_id: &str,
    discord_username: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "guardian")]
    {
        let event = GuardianEvent::LinkWalletDiscord {
            wallet_address: wallet_address.to_string(),
            discord_user_id: discord_user_id.to_string(),
            discord_username: discord_username.to_string(),
        };

        emit_guardian_event(event).await;
    }

    #[cfg(not(feature = "guardian"))]
    {
        // No-op when the `guardian` feature is not enabled.
        let _ = (wallet_address, discord_user_id, discord_username);
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  PUBLIC HELPER FOR GUARDIAN
// ═══════════════════════════════════════════════════════════════════════════

/// Lookup Discord user ID for a given wallet address (used by Guardian)
pub fn lookup_discord_id_for_wallet(db: &Connection, wallet_address: &str) -> Option<String> {
    db.query_row(
        "SELECT discord_user_id FROM discord_links WHERE wallet_address = ?1",
        params![wallet_address],
        |row| row.get(0),
    )
    .ok()
}
