pub mod handlers;
pub mod models;
pub mod repository;
pub mod service;

use crate::AppState;
use axum::{
    Router,
    routing::{get, post},
};

pub fn routes() -> Router<AppState> {
    let routes = Router::new()
        .route("/", post(handlers::create_event).get(handlers::list_events))
        .route("/occurrences", get(handlers::list_occurrences))
        .route(
            "/{id}",
            get(handlers::get_event)
                .patch(handlers::update_event)
                .delete(handlers::delete_event),
        );

    Router::new().nest("/events", routes)
}
