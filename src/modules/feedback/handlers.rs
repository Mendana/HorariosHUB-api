use axum::{Json, extract::State, http::StatusCode};
use validator::Validate;

use crate::{
    AppState,
    errors::{ApiResult, AppError},
    modules::feedback::{
        models::{SubmitFeedbackRequest, SubmitFeedbackResponse},
        service,
    },
};

/// POST /feedback
///
/// Endpoint público (sin autenticación) para el formulario de contacto/
/// sugerencias del front. Reenvía el mensaje por email a la lista de
/// destinatarios configurada en `FEEDBACK_RECIPIENTS`.
#[tracing::instrument(skip(state, payload), fields(from_email = %payload.email))]
pub async fn submit_feedback(
    State(state): State<AppState>,
    Json(payload): Json<SubmitFeedbackRequest>,
) -> ApiResult<(StatusCode, Json<SubmitFeedbackResponse>)> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let response = service::submit_feedback(
        state.email.as_ref(),
        &state.config.feedback_recipients,
        payload,
    )
    .await?;

    Ok((StatusCode::OK, Json(response)))
}
