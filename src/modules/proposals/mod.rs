pub mod handlers;
pub mod models;
pub mod repository;
pub mod service;

use crate::AppState;
use axum::{Router, routing::post};

pub fn routes() -> Router<AppState> {
    let routes = Router::new().route("/", post(handlers::create_proposal));

    Router::new().nest("/proposals", routes)
}
