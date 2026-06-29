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
        classes::{
            models::{
                ClassItem, CreateClassRequest, DeleteClassResponse, ListClassesQueryParams,
                ListClassesResponse, UpdateClassRequest,
            },
            service,
        },
    },
};

/// POST /api/classes
#[tracing::instrument(skip(state, professor), fields(user_id = %professor.id, user_email = %professor.email))]
pub async fn create_class(
    State(state): State<AppState>,
    ProfessorOrAbove(professor): ProfessorOrAbove,
    Json(payload): Json<CreateClassRequest>,
) -> ApiResult<(StatusCode, Json<ClassItem>)> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let response = service::create_class(
        state.class_repo.as_ref(),
        state.proposals_repo.as_ref(),
        payload,
        professor.id,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(response)))
}

/// PATCH /api/classes/{id}
#[tracing::instrument(skip(state, professor), fields(user_id = %professor.id, session_id = %id, user_email = %professor.email))]
pub async fn update_class(
    State(state): State<AppState>,
    ProfessorOrAbove(professor): ProfessorOrAbove,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateClassRequest>,
) -> ApiResult<Json<ClassItem>> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let response = service::update_class(
        state.class_repo.as_ref(),
        state.proposals_repo.as_ref(),
        id,
        professor.id,
        payload,
    )
    .await?;

    Ok(Json(response))
}

/// DELETE /api/classes/{id}
#[tracing::instrument(skip(state, professor), fields(user_id = %professor.id, session_id = %id, user_email = %professor.email))]
pub async fn delete_class(
    State(state): State<AppState>,
    ProfessorOrAbove(professor): ProfessorOrAbove,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<DeleteClassResponse>> {
    let response = service::delete_class(
        state.class_repo.as_ref(),
        state.proposals_repo.as_ref(),
        id,
        professor.id,
    )
    .await?;

    Ok(Json(response))
}

/// GET api/classes
#[tracing::instrument(skip(state, _auth), fields(user_email = %_auth.user.email))]
pub async fn get_classes(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(params): Query<ListClassesQueryParams>,
) -> ApiResult<(StatusCode, Json<ListClassesResponse>)> {
    let response = service::get_classes(state.class_repo.as_ref(), params).await?;
    Ok((StatusCode::OK, Json(response)))
}
