use axum::{
    Router,
    routing::{get, patch},
};

use crate::AppState;

pub mod handlers;
pub mod models;
pub mod repository;
pub mod service;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/notifications", get(handlers::get_notifications))
        .route(
            "/notifications/{id}/read",
            patch(handlers::mark_notification_as_read),
        )
        .route(
            "/notifications/unread-count",
            get(handlers::get_unread_notifications_count),
        )
        .route(
            "/notifications/read-all",
            patch(handlers::mark_all_notifications_as_read),
        )
}
