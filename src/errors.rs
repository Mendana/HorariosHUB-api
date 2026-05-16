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

    #[error("Sin permisos suficientes")]
    Forbidden,

    #[error("Recurso no encontrado")]
    NotFound,

    #[error("Datos inválidos: {0}")]
    BadRequest(String),

    #[error("Conflicto: {0}")]
    Conflict(String),

    #[error("Validación fallida: {0}")]
    Validation(String),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            AppError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            AppError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "validation_error"),
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
