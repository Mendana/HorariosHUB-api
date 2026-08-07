use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};

use crate::{
    AppState,
    errors::ApiResult,
    modules::{
        auth::middleware::AuthenticatedUser,
        notifications::{
            models::{GetNotificationsQuery, GetNotificationsResponse},
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
