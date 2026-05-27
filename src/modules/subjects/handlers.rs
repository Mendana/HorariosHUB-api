use axum::{Json, extract::State};
use reqwest::StatusCode;

use crate::{
    AppState,
    errors::ApiResult,
    modules::{
        auth::middleware::AuthenticatedUser,
        subjects::{
            models::{
                AutoSelectResponse, CatalogResponse, UserSelectionRequest, UserSelectionResponse,
            },
            service,
        },
    },
};

/// GET /subjects/catalog
#[tracing::instrument(skip(state, user), fields(user_id = %user.user.id))]
pub async fn get_catalog_by_user(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<Json<CatalogResponse>> {
    let response = service::get_catalog_by_user(state.subjects_repo.as_ref(), user.user.id).await?;

    Ok(Json(response))
}

/// POST /subjects/selection
#[tracing::instrument(skip(state, user), fields(user_id = %user.user.id))]
pub async fn set_user_selection_destructive(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(payload): Json<UserSelectionRequest>,
) -> ApiResult<Json<UserSelectionResponse>> {
    let response = service::set_user_selection_destructive(
        state.subjects_repo.as_ref(),
        user.user.id,
        payload.groups,
    )
    .await?;

    Ok(Json(response))
}

#[tracing::instrument(skip(state, user), fields(user_id = %user.user.id))]
pub async fn auto_select_subjects(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<(StatusCode, Json<AutoSelectResponse>)> {
    let response = service::auto_select_subjects(
        state.subjects_repo.clone(),
        state.auto_select_semaphore.clone(),
        user.user.id,
        &user.user.email,
        &state.config.scraper_url,
    )
    .await?;

    Ok((StatusCode::ACCEPTED, Json(response)))
}
