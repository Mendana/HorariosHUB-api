use crate::AppState;
use crate::errors::AppError;
use crate::jwt;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::header::{AUTHORIZATION, COOKIE};
use axum::http::request::Parts;

/// Extractor que valida el JWT y extrae los Claims del usuario.
/// Intenta leer el token del header `Authorization: Bearer` primero,
/// y si no está presente, lo busca en la cookie `access_token`.
pub struct AuthenticatedUser {
    pub claims: jwt::Claims,
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);

        let token = extract_bearer(parts).or_else(|| extract_cookie(parts, "access_token"));
        let token = token.ok_or(AppError::Unauthorized)?;

        let claims = jwt::verify_token(&token, &state.config.jwt_secret)?;

        Ok(AuthenticatedUser { claims })
    }
}

fn extract_bearer(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

fn extract_cookie(parts: &Parts, name: &str) -> Option<String> {
    let header = parts.headers.get(COOKIE)?.to_str().ok()?;
    header.split(';').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        if k.trim() == name {
            Some(v.trim().to_string())
        } else {
            None
        }
    })
}
