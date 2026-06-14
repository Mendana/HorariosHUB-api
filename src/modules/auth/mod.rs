pub mod handlers;
pub mod middleware;
pub mod models;
pub mod service;

use crate::{AppState, services::rate_limit};
use axum::{
    Router,
    routing::{get, post},
};

pub fn routes(with_rate_limit: bool) -> Router<AppState> {
    let open_auth_routes = Router::new()
        .route("/register", post(handlers::register))
        .route("/me", get(handlers::get_user))
        .route("/verify", get(handlers::verify_email))
        .route("/reset-password", post(handlers::reset_password))
        .route("/recover", post(handlers::recover_password))
        .route("/logout", post(handlers::logout));

    let login_route = Router::new().route("/login", post(handlers::login));

    let login_route = if with_rate_limit {
        login_route.layer(rate_limit::ip_layer(1, 5))
    } else {
        login_route
    };

    Router::new().nest("/auth", open_auth_routes.merge(login_route))
}
