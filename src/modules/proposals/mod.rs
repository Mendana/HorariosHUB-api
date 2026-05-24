pub mod handlers;
pub mod models;
pub mod repository;
pub mod service;

use crate::AppState;
use axum::{
    Router,
    routing::{patch, post},
};

pub fn routes() -> Router<AppState> {
    let routes = Router::new()
        .route(
            "/",
            post(handlers::create_proposal).get(handlers::list_proposals),
        )
        .route("/{id}/approve", patch(handlers::approve_proposal))
        .route("/{id}/reject", patch(handlers::reject_proposal));

    Router::new().nest("/proposals", routes)
}
