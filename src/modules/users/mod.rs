use axum::{Router, routing::get};

use crate::AppState;

pub mod handlers;
pub mod models;
pub mod repository;
pub mod service;

pub fn routes() -> Router<AppState> {
    let routes = Router::new().route("/", get(handlers::get_all_users));

    Router::new().nest("/users", routes)
}
