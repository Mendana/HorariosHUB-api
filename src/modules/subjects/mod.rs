pub mod handlers;
pub mod models;
pub mod repository;
pub mod service;

use crate::{AppState, services::rate_limit};
use axum::{
    Router,
    routing::{get, post},
};

pub fn routes(with_rate_limit: bool) -> Router<AppState> {
    let catalog_and_selection = Router::new()
        .route("/catalog", get(handlers::get_catalog_by_user))
        .route("/selection", post(handlers::set_user_selection_destructive));

    let auto_select = Router::new().route("/auto-select", post(handlers::auto_select_subjects));

    // Solo se limita por IP, el límite por usuario ya se gestiona internamente
    let auto_select = if with_rate_limit {
        auto_select.layer(rate_limit::ip_layer(5, 10))
    } else {
        auto_select
    };

    Router::new().nest("/subjects", catalog_and_selection.merge(auto_select))
}
