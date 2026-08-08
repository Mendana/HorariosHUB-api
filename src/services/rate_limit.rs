use std::sync::Arc;

use axum::{
    body::Body,
    extract::Request,
    http::header::AUTHORIZATION,
    response::{IntoResponse, Response},
};
use governor::middleware::NoOpMiddleware;
use tower_governor::{
    GovernorError, GovernorLayer,
    governor::GovernorConfigBuilder,
    key_extractor::{KeyExtractor, SmartIpKeyExtractor},
};
use uuid::Uuid;

use crate::errors::AppError;

/// Traduce los errores de `tower_governor` al mismo formato JSON (`{ "error", "message" }`)
/// que usa el resto de la API, en vez del texto plano que devuelve la librería por defecto.
fn rate_limit_error_response(err: GovernorError) -> Response<Body> {
    match err {
        GovernorError::TooManyRequests { .. } => AppError::TooManyRequests.into_response(),
        _ => AppError::Internal(anyhow::anyhow!("Error en el rate limiter: {err}")).into_response(),
    }
}

pub fn ip_layer(
    per_second: u64,
    burst: u32,
) -> GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware, Body> {
    let config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(per_second)
            .burst_size(burst)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .unwrap(),
    );
    GovernorLayer::new(config).error_handler(rate_limit_error_response)
}

#[derive(Clone)]
pub struct UserIdKeyExtractor {
    jwt_secret: String,
}

impl UserIdKeyExtractor {
    pub fn new(jwt_secret: impl Into<String>) -> Self {
        Self {
            jwt_secret: jwt_secret.into(),
        }
    }
}

impl KeyExtractor for UserIdKeyExtractor {
    type Key = Uuid;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        let token = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or(GovernorError::UnableToExtractKey)?;

        let claims = crate::jwt::verify_token(token, &self.jwt_secret)
            .map_err(|_| GovernorError::UnableToExtractKey)?;

        Uuid::parse_str(&claims.sub).map_err(|_| GovernorError::UnableToExtractKey)
    }
}

pub fn user_layer(
    per_second: u64,
    burst: u32,
    jwt_secret: impl Into<String>,
) -> GovernorLayer<UserIdKeyExtractor, NoOpMiddleware, Body> {
    let config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(per_second)
            .burst_size(burst)
            .key_extractor(UserIdKeyExtractor::new(jwt_secret))
            .finish()
            .unwrap(),
    );
    GovernorLayer::new(config).error_handler(rate_limit_error_response)
}
