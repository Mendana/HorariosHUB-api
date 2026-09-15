use crate::common::{login_as, setup};
use axum::http::StatusCode;
use serde_json::Value;

// ─── Autenticación / autorización ─────────────────────────────────────────────

#[tokio::test]
async fn sync_status_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx.server.get("/scraper/sync/status").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn sync_status_devuelve_403_como_student() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student_sync_status@uniovi.es", "student").await;

    let response = ctx
        .server
        .get("/scraper/sync/status")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn sync_status_devuelve_403_como_profesor() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_sync_status@uniovi.es", "professor").await;

    let response = ctx
        .server
        .get("/scraper/sync/status")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

// ─── Casos válidos ────────────────────────────────────────────────────────────

#[tokio::test]
async fn sync_status_devuelve_syncing_false_sin_lock_activo() {
    let ctx = setup().await;
    let token = login_as(&ctx, "admin_sync_status_free@uniovi.es", "admin").await;

    let response = ctx
        .server
        .get("/scraper/sync/status")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: Value = response.json();
    assert_eq!(body["syncing"], false);
    assert!(body["lockedBy"].is_null());
    assert!(body["lockedSince"].is_null());
}

#[tokio::test]
async fn sync_status_devuelve_syncing_true_con_lock_activo() {
    let ctx = setup().await;
    let token = login_as(&ctx, "admin_sync_status_locked@uniovi.es", "admin").await;

    sqlx::query!(
        "INSERT INTO scraper_locks (id, locked_by, locked_at) VALUES ('scraper_run', 'cronjob', NOW())"
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    let response = ctx
        .server
        .get("/scraper/sync/status")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: Value = response.json();
    assert_eq!(body["syncing"], true);
    assert_eq!(body["lockedBy"], "cronjob");
    assert!(body["lockedSince"].is_string());
}

#[tokio::test]
async fn sync_status_ignora_locks_caducados_de_mas_de_dos_horas() {
    let ctx = setup().await;
    let token = login_as(&ctx, "admin_sync_status_stale@uniovi.es", "admin").await;

    sqlx::query!(
        "INSERT INTO scraper_locks (id, locked_by, locked_at) VALUES ('scraper_run', 'cronjob', NOW() - INTERVAL '3 hours')"
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    let response = ctx
        .server
        .get("/scraper/sync/status")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: Value = response.json();
    assert_eq!(body["syncing"], false);
}
