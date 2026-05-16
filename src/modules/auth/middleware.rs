use crate::AppState;
use crate::errors::AppError;
use crate::jwt;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;

/// Extractor que valida el JWT y extrae los Claims del usuario
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
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let token = auth_header
            .as_ref()
            .ok_or(AppError::Unauthorized)?
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;

        let claims = jwt::verify_token(token, &state.config.jwt_secret)?;

        Ok(AuthenticatedUser { claims })
    }
}
