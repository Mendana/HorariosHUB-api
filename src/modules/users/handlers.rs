use axum::{Json, extract::State, http::StatusCode};

use crate::{
    AppState,
    errors::ApiResult,
    modules::{
        auth::middleware::ProfessorOrAbove,
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
