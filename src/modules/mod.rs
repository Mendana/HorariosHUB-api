pub mod auth;
pub mod classes;
pub mod schedule;

use crate::AppState;
use axum::Router;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(auth::routes())
        .merge(schedule::routes())
        .merge(classes::routes())
}
