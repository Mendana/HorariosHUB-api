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
async fn delete_notification_elimina_la_notificacion_propia() {
    let ctx = setup().await;
    let token = login_as(&ctx, "borrar_propia@uniovi.es", "student").await;
    let uid = user_id(&ctx.pool, "borrar_propia@uniovi.es").await;
    let notification_id = insert_notification(&ctx.pool, uid, false).await;

    let response = ctx
        .server
        .delete(&format!("/notifications/{notification_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["message"], "Notificación eliminada");

    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM notifications WHERE id = $1",
        notification_id
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    assert_eq!(exists, Some(0));
}

#[tokio::test]
async fn delete_notification_devuelve_404_si_no_existe() {
    let ctx = setup().await;
    let token = login_as(&ctx, "borrar_no_existe@uniovi.es", "student").await;

    let response = ctx
        .server
        .delete(&format!("/notifications/{}", Uuid::new_v4()))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_notification_devuelve_403_si_es_de_otro_usuario() {
    let ctx = setup().await;
    let _owner_token = login_as(&ctx, "borrar_dueno@uniovi.es", "student").await;
    let owner_id = user_id(&ctx.pool, "borrar_dueno@uniovi.es").await;
    let notification_id = insert_notification(&ctx.pool, owner_id, false).await;

    let intruder_token = login_as(&ctx, "borrar_intruso@uniovi.es", "student").await;

    let response = ctx
        .server
        .delete(&format!("/notifications/{notification_id}"))
        .add_header("Authorization", format!("Bearer {intruder_token}"))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);

    // No debe haberse eliminado
    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM notifications WHERE id = $1",
        notification_id
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    assert_eq!(exists, Some(1));
}

#[tokio::test]
async fn delete_notification_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx
        .server
        .delete(&format!("/notifications/{}", Uuid::new_v4()))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn delete_notification_no_afecta_a_otras_notificaciones_del_mismo_usuario() {
    let ctx = setup().await;
    let token = login_as(&ctx, "borrar_una@uniovi.es", "student").await;
    let uid = user_id(&ctx.pool, "borrar_una@uniovi.es").await;
    let borrar_id = insert_notification(&ctx.pool, uid, false).await;
    let mantener_id = insert_notification(&ctx.pool, uid, false).await;

    ctx.server
        .delete(&format!("/notifications/{borrar_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .assert_status_ok();

    let mantiene = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM notifications WHERE id = $1",
        mantener_id
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    assert_eq!(mantiene, Some(1));
}
