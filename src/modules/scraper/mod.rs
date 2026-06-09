use axum::{Router, routing::post};

use crate::AppState;

pub mod handlers;
pub mod models;
pub mod parser;
pub mod repository;
pub mod service;

pub fn routes() -> Router<AppState> {
    Router::new().route("/scraper/sync", post(handlers::trigger_sync))
}
