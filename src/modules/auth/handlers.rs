use super::models::{RegisterRequest, RegisterResponse};
use super::service;
use crate::errors::AppError;
use crate::{AppState, errors::ApiResult};
use axum::{Json, extract::State, http::StatusCode};
use validator::Validate;

// POST /auth/register
#[tracing::instrument(
    skip(state),                    // No logear el estado completo
    fields(email = %payload.email)  // Logear el email
)]
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> ApiResult<(StatusCode, Json<RegisterResponse>)> {
    payload
        .validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let response = service::register(state.user_repo.as_ref(), payload).await?;
    Ok((StatusCode::CREATED, Json(response)))
}
