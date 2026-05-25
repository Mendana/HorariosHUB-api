pub mod auth;
pub mod classes;
pub mod proposals;
pub mod schedule;
pub mod scraper;
pub mod subjects;

use crate::AppState;
use axum::Router;

pub fn routes(with_rate_limit: bool) -> Router<AppState> {
    Router::new()
        .merge(auth::routes(with_rate_limit))
        .merge(schedule::routes())
        .merge(classes::routes())
        .merge(proposals::routes())
        .merge(subjects::routes())
        .merge(scraper::routes())
}
