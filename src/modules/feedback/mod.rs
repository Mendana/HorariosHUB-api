pub mod handlers;
pub mod models;
pub mod service;

use crate::{AppState, services::rate_limit};
use axum::{Router, routing::post};

pub fn routes(with_rate_limit: bool) -> Router<AppState> {
    let routes = Router::new().route("/", post(handlers::submit_feedback));

    let routes = if with_rate_limit {
        routes.layer(rate_limit::ip_layer(1, 5))
    } else {
        routes
    };

    Router::new().nest("/feedback", routes)
}
