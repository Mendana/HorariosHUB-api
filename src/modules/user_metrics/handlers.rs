/// GET /user-metrics
use axum::{
    Json,
    extract::{Query, State},
};

use crate::{
    AppState,
    errors::ApiResult,
    modules::{
        auth::middleware::AuthenticatedUser,
        user_metrics::{
            models::{GetUserMetricsQuery, GetUserMetricsResponse, WeeklyEvolutionEntry},
            service,
        },
    },
};

#[tracing::instrument(skip(state, user), fields(user_id = %user.user.id, user_email = %user.user.email))]
pub async fn get_user_metrics(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(query_params): Query<GetUserMetricsQuery>,
) -> ApiResult<Json<GetUserMetricsResponse>> {
    let response = service::get_user_metrics(
        state.user_metrics_repo.as_ref(),
        user.user.id,
        query_params.semester,
    )
    .await?;

    Ok(Json(response))
}

/// GET /user-metrics/weekly
#[tracing::instrument(skip(state, user), fields(user_id = %user.user.id, user_email = %user.user.email))]
pub async fn get_weekly_evolution(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(query_params): Query<GetUserMetricsQuery>,
) -> ApiResult<Json<Vec<WeeklyEvolutionEntry>>> {
    let response = service::get_weekly_evolution(
        state.user_metrics_repo.as_ref(),
        user.user.id,
        query_params.semester,
    )
    .await?;

    Ok(Json(response))
}
