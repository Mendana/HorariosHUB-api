pub mod handlers;
pub mod models;
pub mod repository;
pub mod service;

use crate::AppState;
use axum::{Router, routing::get};

pub fn routes() -> Router<AppState> {
    let routes = Router::new().route("/catalog", get(handlers::get_catalog_by_user));

    Router::new().nest("/subjects", routes)
}
