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
async fn patch_read_all_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx.server.patch("/notifications/read-all").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn patch_read_all_devuelve_0_si_no_hay_notificaciones() {
    let ctx = setup().await;
    let token = login_as(&ctx, "read_all_vacio@uniovi.es", "student").await;

    let response = ctx
        .server
        .patch("/notifications/read-all")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["updated"], 0);
}

/// Este es el caso que detecta el bug original: si ya hay notificaciones leídas
/// de antes, `updated` debe contar solo las que pasan de no leída a leída,
/// no el total de notificaciones del usuario.
#[tokio::test]
async fn patch_read_all_cuenta_solo_las_que_estaban_sin_leer() {
    let ctx = setup().await;
    let token = login_as(&ctx, "read_all_mixto@uniovi.es", "student").await;
    let uid = user_id(&ctx.pool, "read_all_mixto@uniovi.es").await;

    insert_notification(&ctx.pool, uid, true).await; // ya leída
    insert_notification(&ctx.pool, uid, true).await; // ya leída
    insert_notification(&ctx.pool, uid, false).await;
    insert_notification(&ctx.pool, uid, false).await;
    insert_notification(&ctx.pool, uid, false).await;

    let response = ctx
        .server
        .patch("/notifications/read-all")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["updated"], 3);
}

#[tokio::test]
async fn patch_read_all_marca_todas_como_leidas_en_bbdd() {
    let ctx = setup().await;
    let token = login_as(&ctx, "read_all_bbdd@uniovi.es", "student").await;
    let uid = user_id(&ctx.pool, "read_all_bbdd@uniovi.es").await;

    insert_notification(&ctx.pool, uid, false).await;
    insert_notification(&ctx.pool, uid, false).await;

    ctx.server
        .patch("/notifications/read-all")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .assert_status_ok();

    let unread_left = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND read = false",
        uid
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(unread_left, Some(0));
}

#[tokio::test]
async fn patch_read_all_es_idempotente_en_llamadas_sucesivas() {
    let ctx = setup().await;
    let token = login_as(&ctx, "read_all_doble@uniovi.es", "student").await;
    let uid = user_id(&ctx.pool, "read_all_doble@uniovi.es").await;

    insert_notification(&ctx.pool, uid, false).await;
    insert_notification(&ctx.pool, uid, false).await;

    let first = ctx
        .server
        .patch("/notifications/read-all")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;
    first.assert_status_ok();
    let first_body: serde_json::Value = first.json();
    assert_eq!(first_body["updated"], 2);

    let second = ctx
        .server
        .patch("/notifications/read-all")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;
    second.assert_status_ok();
    let second_body: serde_json::Value = second.json();
    assert_eq!(second_body["updated"], 0);
}

#[tokio::test]
async fn patch_read_all_no_afecta_a_notificaciones_de_otro_usuario() {
    let ctx = setup().await;
    let token = login_as(&ctx, "read_all_propio@uniovi.es", "student").await;
    let uid = user_id(&ctx.pool, "read_all_propio@uniovi.es").await;
    insert_notification(&ctx.pool, uid, false).await;

    let _otro_token = login_as(&ctx, "read_all_ajeno@uniovi.es", "student").await;
    let otro_uid = user_id(&ctx.pool, "read_all_ajeno@uniovi.es").await;
    let ajena_id = insert_notification(&ctx.pool, otro_uid, false).await;

    ctx.server
        .patch("/notifications/read-all")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .assert_status_ok();

    let ajena_read = sqlx::query_scalar!("SELECT read FROM notifications WHERE id = $1", ajena_id)
        .fetch_one(&ctx.pool)
        .await
        .unwrap();

    assert!(
        !ajena_read,
        "no debe marcar como leída una notificación de otro usuario"
    );
}
