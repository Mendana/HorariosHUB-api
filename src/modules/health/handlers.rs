use axum::{Json, extract::State, http::StatusCode};

use crate::{
    AppState,
    modules::health::{models::HealthResponse, service},
};

pub async fn health_check(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let response = service::check_health(state.health_repo.as_ref()).await;

    let status = if response.db == "ok" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status, Json(response))
}
