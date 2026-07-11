use super::service;
use crate::AppState;
use crate::errors::{ApiResult, AppError};
use crate::modules::auth::middleware::AuthenticatedUser;
use crate::modules::schedule::models::{
    CopyScheduleQuery, CopyScheduleResponse, GetScheduleQuery, GetScheduleResponse,
};
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::{Json, extract::State};

/// GET /api/schedule/{identifier}?start={fechaInicial}
/// GET /api/schedule/{identifier}?month={YYYY-MM}
#[tracing::instrument(skip(state, query), fields(identifier = %identifier))]
pub async fn get_schedule(
    State(state): State<AppState>,
    Path(identifier): Path<String>,
    Query(query): Query<GetScheduleQuery>,
) -> ApiResult<(StatusCode, Json<GetScheduleResponse>)> {
    let schedule = match (query.start, query.month) {
        (Some(start), None) => {
            service::get_user_weekly_schedule(
                state.user_repo.as_ref(),
                state.schedule_repo.as_ref(),
                &identifier,
                start,
            )
            .await?
        }
        (None, Some(ref month)) => {
            service::get_user_monthly_schedule(
                state.user_repo.as_ref(),
                state.schedule_repo.as_ref(),
                &identifier,
                month,
            )
            .await?
        }
        _ => {
            return Err(AppError::BadRequest(
                "Provide either 'start' or 'month', but not both".to_string(),
            ));
        }
    };

    Ok((StatusCode::OK, Json(schedule)))
}

/// POST /api/schedule/copy?user=user_email
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user.id, user_email = %auth.user.email, from_email = %payload.user))]
pub async fn copy_schedule(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(payload): Query<CopyScheduleQuery>,
) -> ApiResult<(StatusCode, Json<CopyScheduleResponse>)> {
    let response = service::copy_schedule(
        state.user_repo.as_ref(),
        state.schedule_repo.as_ref(),
        &payload.user,
        &auth.user,
    )
    .await?;

    Ok((StatusCode::OK, Json(response)))
}
