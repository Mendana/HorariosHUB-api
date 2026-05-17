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
    let auth_routes = Router::new()
        .route("/register", post(handlers::register))
        .route("/me", get(handlers::get_user))
        .route("/login", post(handlers::login))
        .route("/verify", get(handlers::verify_email))
        .route("/reset-password", post(handlers::reset_password))
        .route("/recover", post(handlers::recover_password))
        .route("/logout", post(handlers::logout));

    Router::new().nest("/auth", auth_routes)
}
