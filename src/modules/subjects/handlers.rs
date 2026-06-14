use axum::{
    Json,
    extract::{Path, State},
};
use reqwest::StatusCode;

use crate::{
    AppState,
    errors::ApiResult,
    modules::{
        auth::middleware::AuthenticatedUser,
        subjects::{
            models::{
                AllGroupsPerSubjectResponse, AllSubjectsResponse, AutoSelectResponse,
                AutoSelectStatusResponse, CatalogResponse, UserSelectionRequest,
                UserSelectionResponse,
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

#[tracing::instrument(skip(state, user), fields(user_id = %user.user.id))]
pub async fn get_auto_selection_status(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<(StatusCode, Json<AutoSelectStatusResponse>)> {
    let response =
        service::auto_select_subjects_status(state.subjects_repo.as_ref(), user.user.id).await?;

    Ok((StatusCode::OK, Json(response)))
}

#[tracing::instrument(skip(state), fields(user_id = "N/A"))]
pub async fn get_all_subjects_catalog(
    State(state): State<AppState>,
) -> ApiResult<(StatusCode, Json<AllSubjectsResponse>)> {
    let response = service::get_all_subjects_catalog(state.subjects_repo.as_ref()).await?;

    Ok((
        StatusCode::OK,
        Json(AllSubjectsResponse { subjects: response }),
    ))
}

#[tracing::instrument(skip(state), fields(subject = %subject))]
pub async fn get_all_groups_per_subject(
    State(state): State<AppState>,
    Path(subject): Path<String>,
) -> ApiResult<(StatusCode, Json<AllGroupsPerSubjectResponse>)> {
    let response =
        service::get_all_groups_per_subject(state.subjects_repo.as_ref(), &subject).await?;

    Ok((
        StatusCode::OK,
        Json(AllGroupsPerSubjectResponse { groups: response }),
    ))
}
