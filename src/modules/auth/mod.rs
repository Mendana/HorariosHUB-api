pub mod handlers;
pub mod middleware;
pub mod models;
pub mod service;

use crate::AppState;
use axum::Router;

pub fn routes() -> Router<AppState> {
    Router::new()
    // .route("/auth/login", post(handlers::login))
    // .route("/auth/register", post(handlers::register))
    // .route("/auth/refresh", post(handlers::refresh))
    // .route("/auth/logout", post(handlers::logout))
}
