use axum::{Json, extract::State};

use crate::{
    AppState,
    errors::{ApiResult, AppError},
    modules::{
        auth::middleware::AdminUser,
        scraper::{models::SyncResponse, service},
    },
};

///POST /scraper/sync
///
///Dispara la sincronización manual
#[tracing::instrument(skip(state, admin), fields(user_id = %admin.id))]
pub async fn trigger_sync(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
) -> ApiResult<Json<SyncResponse>> {
    let full_scraper_url = format!("{}/scrape", state.config.scraper_url);

    let result = service::run_sync(
        state.scraper_repo.as_ref(),
        &full_scraper_url,
        state.config.scraper_min_sessions,
        "manual-trigger",
    )
    .await?;

    if result.aborted {
        return Err(AppError::Conflict(
            result
                .abort_reason
                .unwrap_or_else(|| "Sincronización abortada".into()),
        ));
    }

    Ok(Json(SyncResponse::from(result)))
}
