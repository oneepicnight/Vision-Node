use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;

pub struct AdminAuth;

impl<S> FromRequestParts<S> for AdminAuth
where
    S: Send + Sync + 'static,
{
    type Rejection = (StatusCode, &'static str);

    fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let want = std::env::var("ADMIN_TOKEN").ok();
        if let Some(expected) = want {
            let got = parts.headers.get("X-Admin-Token").and_then(|v| v.to_str().ok()).unwrap_or("");
            if got != expected {
                return Err((StatusCode::UNAUTHORIZED, "admin token required"));
            }
        }
        Ok(AdminAuth)
    }
}
