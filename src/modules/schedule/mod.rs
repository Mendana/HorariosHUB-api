pub mod handlers;
pub mod models;
pub mod repository;
pub mod service;

use crate::AppState;
use axum::{Router, routing::get};

pub fn routes() -> Router<AppState> {
    let api_routes = Router::new().route("/schedule/{identifier}", get(handlers::get_schedule));
    Router::new().nest("/api", api_routes)
}
