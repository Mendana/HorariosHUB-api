use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("No autenticado")]
    Unauthorized,

    #[error("Email o contraseña incorrectos")]
    InvalidCredentials,

    #[error("Debes verificar tu email antes de continuar")]
    EmailNotVerified,

    #[error("Sin permisos suficientes")]
    Forbidden,

    #[error("Recurso no encontrado")]
    NotFound,

    #[error("Datos inválidos: {0}")]
    BadRequest(String),

    #[error("Token inválido o ya utilizado")]
    TokenInvalid,

    #[error("El token ha expirado")]
    TokenExpired,

    #[error(
        "La contraseña debe tener al menos 8 caracteres, incluir mayúsculas, minúsculas y números"
    )]
    WeakPassword,

    #[error("El email debe pertenecer al dominio @uniovi.es")]
    InvalidEmailDomain,

    #[error("Conflicto: {0}")]
    Conflict(String),

    #[error("Validación fallida: {0}")]
    Validation(String),

    #[error("Demasiadas solicitudes")]
    TooManyRequests,

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            AppError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "invalid_credentials"),
            AppError::EmailNotVerified => (StatusCode::FORBIDDEN, "email_not_verified"),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            AppError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            AppError::TokenInvalid => (StatusCode::BAD_REQUEST, "token_invalid"),
            AppError::TokenExpired => (StatusCode::BAD_REQUEST, "token_expired"),
            AppError::WeakPassword => (StatusCode::UNPROCESSABLE_ENTITY, "weak_password"),
            AppError::InvalidEmailDomain => {
                (StatusCode::UNPROCESSABLE_ENTITY, "invalid_email_domain")
            }
            AppError::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            AppError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "validation_error"),
            AppError::TooManyRequests => (StatusCode::TOO_MANY_REQUESTS, "too_many_requests"),
            AppError::Database(e) => {
                tracing::error!("DB error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "database_error")
            }
            AppError::Internal(e) => {
                tracing::error!("Internal error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
            }
        };

        // No expoimos detalles de errores internos al cliente, solo un mensaje genérico
        let message = match &self {
            AppError::Database(_) | AppError::Internal(_) => "Error interno del servidor".into(),
            other => other.to_string(),
        };

        (status, Json(json!({ "error": code, "message": message }))).into_response()
    }
}

pub type ApiResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    fn status(err: AppError) -> StatusCode {
        err.into_response().status()
    }

    #[test]
    fn unauthorized_returns_401() {
        assert_eq!(status(AppError::Unauthorized), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn invalid_credentials_returns_401() {
        assert_eq!(
            status(AppError::InvalidCredentials),
            StatusCode::UNAUTHORIZED
        );
    }

    #[test]
    fn forbidden_returns_403() {
        assert_eq!(status(AppError::Forbidden), StatusCode::FORBIDDEN);
    }

    #[test]
    fn email_not_verified_returns_403() {
        assert_eq!(status(AppError::EmailNotVerified), StatusCode::FORBIDDEN);
    }

    #[test]
    fn not_found_returns_404() {
        assert_eq!(status(AppError::NotFound), StatusCode::NOT_FOUND);
    }

    #[test]
    fn bad_request_returns_400() {
        assert_eq!(
            status(AppError::BadRequest("error".into())),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn token_invalid_returns_400() {
        assert_eq!(status(AppError::TokenInvalid), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn token_expired_returns_400() {
        assert_eq!(status(AppError::TokenExpired), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn weak_password_returns_422() {
        assert_eq!(
            status(AppError::WeakPassword),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    #[test]
    fn invalid_email_domain_returns_422() {
        assert_eq!(
            status(AppError::InvalidEmailDomain),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    #[test]
    fn conflict_returns_409() {
        assert_eq!(
            status(AppError::Conflict("conflict".into())),
            StatusCode::CONFLICT
        );
    }

    #[test]
    fn validation_returns_422() {
        assert_eq!(
            status(AppError::Validation("invalid".into())),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    #[test]
    fn internal_returns_500() {
        assert_eq!(
            status(AppError::Internal(anyhow::anyhow!("error interno"))),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn database_returns_500() {
        assert_eq!(
            status(AppError::Database(sqlx::Error::RowNotFound)),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
