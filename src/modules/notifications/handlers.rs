use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    AppState,
    errors::ApiResult,
    modules::{
        auth::middleware::AuthenticatedUser,
        notifications::{
            models::{
                GetNotificationsQuery, GetNotificationsResponse, MarkAllNotificationsReadResponse,
                MarkNotificationReadResponse, UnreadNotificationsCountResponse,
            },
            service,
        },
    },
};

/// GET /notifications
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user.id, user_email = %auth.user.email))]
pub async fn get_notifications(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(params): Query<GetNotificationsQuery>,
) -> ApiResult<(StatusCode, Json<GetNotificationsResponse>)> {
    let response = service::get_notifications(
        state.notifications_repo.as_ref(),
        auth.user.id,
        params.read,
        params.page,
        params.limit,
    )
    .await?;

    Ok((StatusCode::OK, Json(response)))
}

///GET /notifications/unread-count
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user.id, user_email = %auth.user.email))]
pub async fn get_unread_notifications_count(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> ApiResult<Json<UnreadNotificationsCountResponse>> {
    let response =
        service::get_unread_count(state.notifications_repo.as_ref(), auth.user.id).await?;

    Ok(Json(response))
}

/// PATCH /notifications/{id}/read
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user.id, user_email = %auth.user.email, notification_id = %id))]
pub async fn mark_notification_as_read(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<MarkNotificationReadResponse>> {
    let response =
        service::mark_notification_as_read(state.notifications_repo.as_ref(), id, auth.user.id)
            .await?;

    Ok(Json(response))
}

/// PATCH /notifications/read-all
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user.id, user_email = %auth.user.email))]
pub async fn mark_all_notifications_as_read(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> ApiResult<Json<MarkAllNotificationsReadResponse>> {
    let response =
        service::mark_all_notifications_as_read(state.notifications_repo.as_ref(), auth.user.id)
            .await?;

    Ok(Json(response))
}
