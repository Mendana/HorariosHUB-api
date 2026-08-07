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
async fn get_unread_count_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx.server.get("/notifications/unread-count").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_unread_count_devuelve_0_si_no_hay_notificaciones() {
    let ctx = setup().await;
    let token = login_as(&ctx, "sin_no_leidas@uniovi.es", "student").await;

    let response = ctx
        .server
        .get("/notifications/unread-count")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["unreadCount"], 0);
}

#[tokio::test]
async fn get_unread_count_cuenta_solo_las_no_leidas_del_usuario() {
    let ctx = setup().await;
    let token = login_as(&ctx, "conteo@uniovi.es", "student").await;
    let uid = user_id(&ctx.pool, "conteo@uniovi.es").await;

    insert_notification(&ctx.pool, uid, false).await;
    insert_notification(&ctx.pool, uid, false).await;
    insert_notification(&ctx.pool, uid, true).await; // leída: no debe contar

    let otro_token = login_as(&ctx, "conteo_otro@uniovi.es", "student").await;
    let otro_uid = user_id(&ctx.pool, "conteo_otro@uniovi.es").await;
    let _ = otro_token;
    insert_notification(&ctx.pool, otro_uid, false).await; // de otro usuario: no debe contar

    let response = ctx
        .server
        .get("/notifications/unread-count")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["unreadCount"], 2);
}

#[tokio::test]
async fn get_unread_count_baja_tras_marcar_como_leida() {
    let ctx = setup().await;
    let token = login_as(&ctx, "baja_tras_leer@uniovi.es", "student").await;
    let uid = user_id(&ctx.pool, "baja_tras_leer@uniovi.es").await;

    let notification_id = insert_notification(&ctx.pool, uid, false).await;
    insert_notification(&ctx.pool, uid, false).await;

    ctx.server
        .patch(&format!("/notifications/{notification_id}/read"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .assert_status_ok();

    let response = ctx
        .server
        .get("/notifications/unread-count")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["unreadCount"], 1);
}
