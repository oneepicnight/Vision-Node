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
