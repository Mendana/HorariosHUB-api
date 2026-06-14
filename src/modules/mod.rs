pub mod auth;
pub mod classes;
pub mod proposals;
pub mod schedule;
pub mod scraper;
pub mod subjects;
pub mod users;

use crate::AppState;
use axum::Router;

pub fn routes(with_rate_limit: bool) -> Router<AppState> {
    Router::new()
        .merge(auth::routes(with_rate_limit))
        .merge(schedule::routes())
        .merge(classes::routes())
        .merge(proposals::routes())
        .merge(subjects::routes(with_rate_limit))
        .merge(scraper::routes())
        .merge(users::routes())
}
