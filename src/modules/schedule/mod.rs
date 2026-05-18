pub mod handlers;
pub mod models;
pub mod queries;
pub mod service;

use crate::AppState;
use axum::Router;

pub fn routes() -> Router<AppState> {
    Router::new()
}
