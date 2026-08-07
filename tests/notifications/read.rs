use crate::common::{login_as, setup};
use axum::http::StatusCode;
use uuid::Uuid;

async fn insert_notification(pool: &sqlx::PgPool, user_id: Uuid, read: bool) -> Uuid {
    sqlx::query_scalar!(
        r#"
        INSERT INTO notifications (user_id, type, title, body, read)
        VALUES ($1, 'session_modified', 'Clase modificada', 'ALG Teoría ha cambiado de aula', $2)
        RETURNING id
        "#,
        user_id,
        read
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn user_id(pool: &sqlx::PgPool, email: &str) -> Uuid {
    sqlx::query_scalar!("SELECT id FROM users WHERE email = $1", email)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn patch_notifications_read_marca_como_leida() {
    let ctx = setup().await;
    let token = login_as(&ctx, "propio_read@uniovi.es", "student").await;
    let uid = user_id(&ctx.pool, "propio_read@uniovi.es").await;
    let notification_id = insert_notification(&ctx.pool, uid, false).await;

    let response = ctx
        .server
        .patch(&format!("/notifications/{notification_id}/read"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["id"], notification_id.to_string());
    assert_eq!(body["read"], true);

    let read_in_db = sqlx::query_scalar!(
        "SELECT read FROM notifications WHERE id = $1",
        notification_id
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    assert!(read_in_db);
}

#[tokio::test]
async fn patch_notifications_read_es_idempotente_si_ya_estaba_leida() {
    let ctx = setup().await;
    let token = login_as(&ctx, "ya_leida@uniovi.es", "student").await;
    let uid = user_id(&ctx.pool, "ya_leida@uniovi.es").await;
    let notification_id = insert_notification(&ctx.pool, uid, true).await;

    let response = ctx
        .server
        .patch(&format!("/notifications/{notification_id}/read"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["read"], true);
}

#[tokio::test]
async fn patch_notifications_read_devuelve_404_si_no_existe() {
    let ctx = setup().await;
    let token = login_as(&ctx, "no_existe@uniovi.es", "student").await;

    let response = ctx
        .server
        .patch(&format!("/notifications/{}/read", Uuid::new_v4()))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn patch_notifications_read_devuelve_403_si_es_de_otro_usuario() {
    let ctx = setup().await;
    let _owner_token = login_as(&ctx, "dueno@uniovi.es", "student").await;
    let owner_id = user_id(&ctx.pool, "dueno@uniovi.es").await;
    let notification_id = insert_notification(&ctx.pool, owner_id, false).await;

    let intruder_token = login_as(&ctx, "intruso@uniovi.es", "student").await;

    let response = ctx
        .server
        .patch(&format!("/notifications/{notification_id}/read"))
        .add_header("Authorization", format!("Bearer {intruder_token}"))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);

    // No debe haberse modificado
    let read_in_db = sqlx::query_scalar!(
        "SELECT read FROM notifications WHERE id = $1",
        notification_id
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    assert!(!read_in_db);
}

#[tokio::test]
async fn patch_notifications_read_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx
        .server
        .patch(&format!("/notifications/{}/read", Uuid::new_v4()))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}
