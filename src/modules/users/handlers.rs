use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{
    AppState,
    errors::ApiResult,
    modules::{
        auth::middleware::{AdminUser, ProfessorOrAbove},
        users::{models::UsersListResponse, service},
    },
};

#[tracing::instrument(
    name = "Get all users",
    skip(state, professor),
    fields(user_email = %professor.email, user_role = ?professor.role)
)]
pub async fn get_all_users(
    State(state): State<AppState>,
    ProfessorOrAbove(professor): ProfessorOrAbove,
) -> ApiResult<(StatusCode, Json<UsersListResponse>)> {
    let users = service::get_all_users(state.user_repo.as_ref()).await?;

    Ok((StatusCode::OK, Json(UsersListResponse { users })))
}

#[tracing::instrument(
    name = "Change user role",
    skip(state, admin),
    fields(user_email = %admin.email, user_role = ?admin.role)
)]
pub async fn change_user_role(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path((identifier, role)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    service::change_user_role(state.user_repo.as_ref(), &identifier, &role).await?;

    Ok(StatusCode::OK)
}

#[tracing::instrument(
    name = "Delete user",
    skip(state, admin),
    fields(user_email = %admin.email, user_role = ?admin.role)
)]
pub async fn delete_user(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(identifier): Path<String>,
) -> ApiResult<StatusCode> {
    service::delete_user(state.user_repo.as_ref(), &identifier).await?;

    Ok(StatusCode::NO_CONTENT)
}
