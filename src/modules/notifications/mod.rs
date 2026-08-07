use axum::{Router, routing::get};

use crate::AppState;

pub mod handlers;
pub mod models;
pub mod repository;
pub mod service;

pub fn routes() -> Router<AppState> {
    Router::new().route("/notifications", get(handlers::get_notifications))
}
