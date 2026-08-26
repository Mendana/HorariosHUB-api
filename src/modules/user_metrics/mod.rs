pub mod handlers;
pub mod models;
pub mod repository;
pub mod service;

use crate::AppState;
use axum::{Router, routing::get};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/user-metrics", get(handlers::get_user_metrics))
        .route("/user-metrics/weekly", get(handlers::get_weekly_evolution))
}
