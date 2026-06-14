use axum::{
    Router,
    routing::{delete, get, patch},
};

use crate::AppState;

pub mod handlers;
pub mod models;
pub mod repository;
pub mod service;

pub fn routes() -> Router<AppState> {
    let routes = Router::new()
        .route("/", get(handlers::get_all_users))
        .route(
            "/{identifier}/change/{role}",
            patch(handlers::change_user_role),
        )
        .route("/{identifier}", delete(handlers::delete_user));

    Router::new().nest("/users", routes)
}
