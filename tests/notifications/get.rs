use crate::common::{login_as, setup};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use uuid::Uuid;

async fn insert_notification(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    read: bool,
    created_at: DateTime<Utc>,
) -> Uuid {
    sqlx::query_scalar!(
        r#"
        INSERT INTO notifications (user_id, type, title, body, read, created_at)
        VALUES ($1, 'session_modified', 'Clase modificada', 'ALG Teoría ha cambiado de aula', $2, $3)
        RETURNING id
        "#,
        user_id,
        read,
        created_at
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
async fn get_notifications_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx.server.get("/notifications").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_notifications_devuelve_200_y_lista_vacia_si_no_hay_notificaciones() {
    let ctx = setup().await;
    let token = login_as(&ctx, "sin_notis@uniovi.es", "student").await;

    let response = ctx
        .server
        .get("/notifications")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["pagination"]["page"], 1);
    assert_eq!(body["pagination"]["limit"], 20);
    assert_eq!(body["pagination"]["total"], 0);
    assert_eq!(body["pagination"]["totalPages"], 0);
}

#[tokio::test]
async fn get_notifications_devuelve_las_propias_ordenadas_por_fecha_desc() {
    let ctx = setup().await;
    let token = login_as(&ctx, "propio@uniovi.es", "student").await;
    let uid = user_id(&ctx.pool, "propio@uniovi.es").await;

    let otro_token = login_as(&ctx, "otro@uniovi.es", "student").await;
    let otro_uid = user_id(&ctx.pool, "otro@uniovi.es").await;
    let _ = otro_token;

    let t0 = Utc::now() - chrono::Duration::minutes(10);
    let older = insert_notification(&ctx.pool, uid, false, t0).await;
    let newer = insert_notification(&ctx.pool, uid, false, t0 + chrono::Duration::minutes(5)).await;
    // Notificación de otro usuario: no debe aparecer
    insert_notification(&ctx.pool, otro_uid, false, t0).await;

    let response = ctx
        .server
        .get("/notifications")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let data = body["data"].as_array().unwrap();

    assert_eq!(data.len(), 2);
    assert_eq!(data[0]["id"], newer.to_string());
    assert_eq!(data[1]["id"], older.to_string());
    assert_eq!(data[0]["type"], "session_modified");
    assert_eq!(data[0]["sessionId"], serde_json::Value::Null);
    assert_eq!(body["pagination"]["total"], 2);
    assert!(data[0].get("userId").is_none(), "no debe exponer user_id");
}

#[tokio::test]
async fn get_notifications_filtra_por_read() {
    let ctx = setup().await;
    let token = login_as(&ctx, "filtro_read@uniovi.es", "student").await;
    let uid = user_id(&ctx.pool, "filtro_read@uniovi.es").await;

    let now = Utc::now();
    let leida = insert_notification(&ctx.pool, uid, true, now).await;
    let no_leida = insert_notification(&ctx.pool, uid, false, now).await;

    let unread_response = ctx
        .server
        .get("/notifications?read=false")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;
    unread_response.assert_status_ok();
    let unread_body: serde_json::Value = unread_response.json();
    let unread_data = unread_body["data"].as_array().unwrap();
    assert_eq!(unread_data.len(), 1);
    assert_eq!(unread_data[0]["id"], no_leida.to_string());

    let read_response = ctx
        .server
        .get("/notifications?read=true")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;
    read_response.assert_status_ok();
    let read_body: serde_json::Value = read_response.json();
    let read_data = read_body["data"].as_array().unwrap();
    assert_eq!(read_data.len(), 1);
    assert_eq!(read_data[0]["id"], leida.to_string());

    let all_response = ctx
        .server
        .get("/notifications")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;
    let all_body: serde_json::Value = all_response.json();
    assert_eq!(all_body["data"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn get_notifications_pagina_respeta_page_y_limit() {
    let ctx = setup().await;
    let token = login_as(&ctx, "paginacion@uniovi.es", "student").await;
    let uid = user_id(&ctx.pool, "paginacion@uniovi.es").await;

    let now = Utc::now();
    for i in 0..5 {
        insert_notification(&ctx.pool, uid, false, now + chrono::Duration::seconds(i)).await;
    }

    let response = ctx
        .server
        .get("/notifications?page=2&limit=2")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert_eq!(body["pagination"]["page"], 2);
    assert_eq!(body["pagination"]["limit"], 2);
    assert_eq!(body["pagination"]["total"], 5);
    assert_eq!(body["pagination"]["totalPages"], 3);
}

#[tokio::test]
async fn get_notifications_limit_se_limita_a_50() {
    let ctx = setup().await;
    let token = login_as(&ctx, "limite@uniovi.es", "student").await;

    let response = ctx
        .server
        .get("/notifications?limit=1000")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["pagination"]["limit"], 50);
}
