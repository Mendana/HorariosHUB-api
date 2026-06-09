use super::service;
use crate::AppState;
use crate::errors::{ApiResult, AppError};
use crate::modules::auth::middleware::AuthenticatedUser;
use crate::modules::schedule::models::{
    CopyScheduleQuery, CopyScheduleResponse, CopyScheduleUsers, GetScheduleQuery,
    GetScheduleResponse,
};
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::{Json, extract::State};

/// GET /api/schedule/{identifier}?start={fechaInicial}
/// Identificador < 6 caracteres -> 400 Bad Request
/// Identificador no existe -> 404 Not Found
/// Identificador existe -> 200 OK con array de sesiones
/// Identificador no tiene datos -> 200 OK con array vacío (caso particular del caso anterior)
#[tracing::instrument(skip(state), fields(identifier = %identifier, start = %query.start))]
pub async fn get_schedule(
    State(state): State<AppState>,
    Path(identifier): Path<String>,
    Query(query): Query<GetScheduleQuery>,
) -> ApiResult<(StatusCode, Json<GetScheduleResponse>)> {
    if identifier.chars().count() < 6 {
        return Err(AppError::BadRequest(format!(
            "Invalid identifier: {identifier}"
        )));
    }

    let user = state.user_repo.find_by_email(&identifier).await?;

    let Some(user) = user else {
        return Err(AppError::NotFound);
    };

    let schedule =
        service::get_user_weekly_schedule(state.schedule_repo.as_ref(), &user.id, query.start)
            .await?;
    Ok((StatusCode::OK, Json(schedule)))
}

/// POST /api/schedule/copy?user=user_email
/// Copia el horario de otro usuario al propio. Se asume que el usuario autenticado existe
/// from_user < 6 caracteres -> 400 Bad Request
/// from_user no existe -> 404 Not Found
/// from_user existe -> 200 OK con mensaje de éxito
/// from_user == to_user -> 400 Bad Request
pub async fn copy_schedule(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(payload): Query<CopyScheduleQuery>,
) -> ApiResult<(StatusCode, Json<CopyScheduleResponse>)> {
    let from_user = payload.user;
    let to_user = auth.user;

    if from_user == to_user.email {
        return Err(AppError::BadRequest(
            "Cannot copy schedule to self".to_string(),
        ));
    }

    if from_user.chars().count() < 6 {
        return Err(AppError::BadRequest(format!("Invalid user: {from_user}")));
    }

    let from_user = state.user_repo.find_by_email(&from_user).await?;

    let Some(from_user) = from_user else {
        return Err(AppError::NotFound);
    };

    let response = service::copy_schedule(
        state.schedule_repo.as_ref(),
        &CopyScheduleUsers {
            from_user: from_user.id,
            to_user: to_user.id,
        },
    )
    .await?;

    Ok((StatusCode::OK, Json(response)))
}
