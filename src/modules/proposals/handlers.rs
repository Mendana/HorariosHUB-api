use axum::{
    Json,
    extract::{Path, Query, State},
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
            models::{
                ApproveProposalResponse, CreateProposalRequest, CreateProposalResponse,
                ListMineProposalsQuery, ListProposalsQuery, ListProposalsResponse,
                RejectProposalResponse,
            },
            service,
        },
    },
};

#[tracing::instrument(skip(state, auth, payload), fields(user_id = %auth.user.id, user_email = %auth.user.email, change_type = %payload.change_type()))]
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
        state.notifications_repo.as_ref(),
        &state.email_queue,
        payload,
        auth.user.id,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(response)))
}

#[tracing::instrument(skip(state, professor), fields(user_id = %professor.id, user_email = %professor.email, proposal_id = %id))]
pub async fn approve_proposal(
    State(state): State<AppState>,
    ProfessorOrAbove(professor): ProfessorOrAbove,
    Path(id): Path<Uuid>,
) -> ApiResult<(StatusCode, Json<ApproveProposalResponse>)> {
    let response = service::approve_proposal(
        state.proposals_repo.as_ref(),
        state.class_repo.as_ref(),
        state.notifications_repo.as_ref(),
        &state.email_queue,
        id,
        professor.id,
    )
    .await?;

    Ok((StatusCode::OK, Json(response)))
}

#[tracing::instrument(skip(state, professor), fields(user_id = %professor.id, user_email = %professor.email, proposal_id = %id))]
pub async fn reject_proposal(
    State(state): State<AppState>,
    ProfessorOrAbove(professor): ProfessorOrAbove,
    Path(id): Path<Uuid>,
) -> ApiResult<(StatusCode, Json<RejectProposalResponse>)> {
    let response = service::reject_proposal(
        state.proposals_repo.as_ref(),
        state.notifications_repo.as_ref(),
        &state.email_queue,
        id,
    )
    .await?;

    Ok((StatusCode::OK, Json(response)))
}

/// GET /proposals?status=pending&page=1&limit=10
///
/// Devulve una lista paginada de propuestas,
/// filtradas por estado (pending, approved, rejected).
#[tracing::instrument(skip(state, professor), fields(user_id = %professor.id, user_email = %professor.email))]
pub async fn list_proposals(
    State(state): State<AppState>,
    ProfessorOrAbove(professor): ProfessorOrAbove,
    Query(params): Query<ListProposalsQuery>,
) -> ApiResult<(StatusCode, Json<ListProposalsResponse>)> {
    let response = service::list_proposals(
        state.proposals_repo.as_ref(),
        params.status,
        params.page.unwrap_or(1),
        params.limit.unwrap_or(10),
    )
    .await?;

    Ok((StatusCode::OK, Json(response)))
}

/// GET /proposals/mine?page=1&limit=10
///
/// Devuelve una lista paginada de propuestas creadas por el usuario autenticado.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user.id, user_email = %auth.user.email))]
pub async fn list_my_proposals(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(params): Query<ListMineProposalsQuery>,
) -> ApiResult<(StatusCode, Json<ListProposalsResponse>)> {
    let user_id = auth.user.id;

    let response = service::list_my_proposals(
        state.proposals_repo.as_ref(),
        user_id,
        params.page.unwrap_or(1),
        params.limit.unwrap_or(10),
    )
    .await?;

    Ok((StatusCode::OK, Json(response)))
}
