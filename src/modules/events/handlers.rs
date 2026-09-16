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
        events::{
            models::{
                CreateEventRequest, DeleteEventResponse, EventItem, ListEventsQuery,
                ListEventsResponse, ListOccurrencesQuery, ListOccurrencesResponse,
                UpdateEventRequest,
            },
            service,
        },
    },
};

/// POST /events
#[tracing::instrument(skip(state, professor, payload), fields(user_id = %professor.id, user_email = %professor.email))]
pub async fn create_event(
    State(state): State<AppState>,
    ProfessorOrAbove(professor): ProfessorOrAbove,
    Json(payload): Json<CreateEventRequest>,
) -> ApiResult<(StatusCode, Json<EventItem>)> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let response = service::create_event(state.event_repo.as_ref(), payload, professor.id).await?;

    Ok((StatusCode::CREATED, Json(response)))
}

/// PATCH /events/{id}
#[tracing::instrument(skip(state, professor, payload), fields(user_id = %professor.id, event_id = %id))]
pub async fn update_event(
    State(state): State<AppState>,
    ProfessorOrAbove(professor): ProfessorOrAbove,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateEventRequest>,
) -> ApiResult<Json<EventItem>> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let response = service::update_event(state.event_repo.as_ref(), id, payload).await?;

    Ok(Json(response))
}

/// DELETE /events/{id}
#[tracing::instrument(skip(state, professor), fields(user_id = %professor.id, event_id = %id))]
pub async fn delete_event(
    State(state): State<AppState>,
    ProfessorOrAbove(professor): ProfessorOrAbove,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<DeleteEventResponse>> {
    let response = service::delete_event(state.event_repo.as_ref(), id).await?;

    Ok(Json(response))
}

/// GET /events/{id}
#[tracing::instrument(skip(state, _auth), fields(user_email = %_auth.user.email))]
pub async fn get_event(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<EventItem>> {
    let response = service::get_event(state.event_repo.as_ref(), id).await?;
    Ok(Json(response))
}

/// GET /events
#[tracing::instrument(skip(state, _auth), fields(user_email = %_auth.user.email))]
pub async fn list_events(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(params): Query<ListEventsQuery>,
) -> ApiResult<Json<ListEventsResponse>> {
    params
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let response = service::list_events(state.event_repo.as_ref(), params).await?;
    Ok(Json(response))
}

/// GET /events/occurrences?from=&to=&subject=
#[tracing::instrument(skip(state, _auth), fields(user_email = %_auth.user.email))]
pub async fn list_occurrences(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(params): Query<ListOccurrencesQuery>,
) -> ApiResult<Json<ListOccurrencesResponse>> {
    params
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let response = service::list_occurrences(state.event_repo.as_ref(), params).await?;
    Ok(Json(response))
}
