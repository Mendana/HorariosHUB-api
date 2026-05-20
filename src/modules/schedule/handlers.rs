use super::service;
use crate::AppState;
use crate::errors::{ApiResult, AppError};
use crate::modules::schedule::models::{ScheduleQuery, ScheduleResponse};
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::{Json, extract::State};

/// GET /api/schedule/{identifier}?start={fechaInicial}
/// Identificador < 6 caracteres -> 400 Bad Request
/// Identificador no existe -> 200 OK con array vacío
/// Identificador existe -> 200 OK con array de sesiones
/// Identificador no tiene datos -> 200 OK con array vacío (caso particular del caso anterior)
#[tracing::instrument(skip(state), fields(identifier = %identifier, start = %query.start))]
pub async fn get_schedule(
    State(state): State<AppState>,
    Path(identifier): Path<String>,
    Query(query): Query<ScheduleQuery>,
) -> ApiResult<(StatusCode, Json<ScheduleResponse>)> {
    if identifier.chars().count() < 6 {
        return Err(AppError::BadRequest(format!(
            "Invalid identifier: {identifier}"
        )));
    }

    let user = state.user_repo.find_by_email(&identifier).await?;

    let Some(user) = user else {
        return Ok((StatusCode::OK, Json(ScheduleResponse::default())));
    };

    let schedule =
        service::get_user_weekly_schedule(state.schedule_repo.as_ref(), &user.id, query.start)
            .await?;
    Ok((StatusCode::OK, Json(schedule)))
}
