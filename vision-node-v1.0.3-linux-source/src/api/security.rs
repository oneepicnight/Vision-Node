/// MAINNET SECURITY LAYER
/// Localhost-only enforcement and admin token verification for sensitive operations
use axum::http::StatusCode;
use std::net::SocketAddr;

/// Check if the request comes from localhost (127.0.0.1 or ::1)
pub fn is_localhost(addr: &SocketAddr) -> bool {
    let ip = addr.ip();
    ip.is_loopback()
}

/// Check if the request has a valid admin token
/// Reads from X-Admin-Token header and compares to VISION_ADMIN_TOKEN env var
pub fn verify_admin_token(token: Option<&str>) -> bool {
    let expected = match std::env::var("VISION_ADMIN_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => {
            tracing::warn!("[SECURITY] VISION_ADMIN_TOKEN not configured");
            return false;
        }
    };

    match token {
        Some(t) if t == expected => true,
        _ => false,
    }
}

/// Combined security check: localhost + admin token
pub fn verify_secure_access(
    addr: &SocketAddr,
    token: Option<&str>,
) -> Result<(), (StatusCode, &'static str)> {
    if !is_localhost(addr) {
        return Err((StatusCode::FORBIDDEN, "Access denied: not localhost"));
    }

    if !verify_admin_token(token) {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Access denied: invalid admin token",
        ));
    }

    Ok(())
}

/// Check if P2P debug endpoints are enabled via env flag
/// Truthy values: "1" | "true" | "yes" (case-insensitive)
pub fn p2p_debug_enabled() -> bool {
    match std::env::var("VISION_ENABLE_P2P_DEBUG") {
        Ok(v) => {
            let v = v.to_lowercase();
            v == "1" || v == "true" || v == "yes"
        }
        Err(_) => false,
    }
}

/// Verify access for P2P debug endpoints, returning 404 on any failure
/// Conditions:
/// - Debug flag enabled
/// - Request from localhost
/// - Valid admin token present
pub fn verify_p2p_debug_access(
    addr: &SocketAddr,
    token: Option<&str>,
) -> Result<(), (StatusCode, &'static str)> {
    if !p2p_debug_enabled() {
        return Err((StatusCode::NOT_FOUND, "not found"));
    }
    if !is_localhost(addr) {
        return Err((StatusCode::NOT_FOUND, "not found"));
    }
    if !verify_admin_token(token) {
        return Err((StatusCode::NOT_FOUND, "not found"));
    }
    Ok(())
}
