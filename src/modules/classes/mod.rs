pub mod handlers;
pub mod models;
pub mod repository;
pub mod service;

use crate::AppState;
use axum::{
    Router,
    routing::{get, patch},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/classes",
            get(handlers::get_classes).post(handlers::create_class),
        )
        .route(
            "/classes/{id}",
            patch(handlers::update_class).delete(handlers::delete_class),
        )
}
