use axum::{Json, extract::State, http::StatusCode};
use validator::Validate;

use crate::{
    AppState,
    errors::{ApiResult, AppError},
    modules::{
        auth::middleware::AuthenticatedUser,
        proposals::{
            models::{CreateProposalRequest, CreateProposalResponse},
            service,
        },
    },
};

#[tracing::instrument(skip(state, auth, payload), fields(user_id = %auth.user.id, change_type = %payload.change_type()))]
pub async fn create_proposal(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(payload): Json<CreateProposalRequest>,
) -> ApiResult<(StatusCode, Json<CreateProposalResponse>)> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let response = service::create_proposal(
        state.proposals_repo.as_ref(),
        state.class_repo.as_ref(),
        payload,
        auth.user.id,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(response)))
}
