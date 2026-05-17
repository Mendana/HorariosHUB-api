pub mod handlers;
pub mod middleware;
pub mod models;
pub mod repository;
pub mod service;

use crate::AppState;
use axum::{
    Router,
    routing::{get, post},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(handlers::register))
        .route("/auth/me", get(handlers::get_user))
        .route("/auth/login", post(handlers::login))
        .route("/auth/verify", get(handlers::verify_email))
}
