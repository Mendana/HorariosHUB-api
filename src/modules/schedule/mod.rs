pub mod handlers;
pub mod models;
pub mod repository;
pub mod service;

use crate::AppState;
use axum::{Router, routing::get};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/schedule/{identifier}", get(handlers::get_schedule))
        .route(
            "/schedule/copy",
            axum::routing::post(handlers::copy_schedule),
        )
}
