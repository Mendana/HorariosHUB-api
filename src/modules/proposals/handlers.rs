use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    AppState,
    errors::{ApiResult, AppError},
    modules::{
        auth::middleware::{AuthenticatedUser, ProfessorOrAbove},
        proposals::{
            models::{ApproveProposalResponse, CreateProposalRequest, CreateProposalResponse},
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

#[tracing::instrument(skip(state, professor), fields(user_id = %professor.id))]
pub async fn approve_proposal(
    State(state): State<AppState>,
    ProfessorOrAbove(professor): ProfessorOrAbove,
    Path(id): Path<Uuid>,
) -> ApiResult<(StatusCode, Json<ApproveProposalResponse>)> {
    let response = service::approve_proposal(
        state.proposals_repo.as_ref(),
        state.class_repo.as_ref(),
        id,
        professor.id,
    )
    .await?;

    Ok((StatusCode::OK, Json(response)))
}
