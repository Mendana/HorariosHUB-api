use axum::{Json, extract::State};

use crate::{
    AppState,
    errors::ApiResult,
    modules::{
        auth::middleware::AuthenticatedUser,
        subjects::{models::CatalogResponse, service},
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
