use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde_json::json;

use crate::common::{login_user, setup, verify_user};

#[tokio::test]
async fn copy_schedule_devuelve_400_si_user_no_valido() {
    let ctx = setup().await;

    let email = "short@uniovi.es";
    let password = "Password123";
    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": email, "password": password }))
        .await;
    verify_user(&ctx.pool, email).await;
    let token = login_user(&ctx.server, email, password).await;

    let response = ctx
        .server
        .post("/schedule/copy?user=short")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_bad_request();
}

#[tokio::test]
async fn copy_schedule_devuelve_404_si_user_no_existe() {
    let ctx = setup().await;

    let email = "usuario@uniovi.es";
    let password = "Password123";
    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": email, "password": password }))
        .await;
    verify_user(&ctx.pool, email).await;
    let token = login_user(&ctx.server, email, password).await;

    let response = ctx
        .server
        .post("/schedule/copy?user=noexiste@uniovi.es")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_not_found();
}

#[tokio::test]
async fn copy_schedule_devuelve_200_y_copia_si_existe() {
    let ctx = setup().await;

    let from_email = "usuario1@uniovi.es";
    let to_email = "usuario2@uniovi.es";
    let password = "Password123";

    // Registrar ambos usuarios
    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": from_email, "password": password }))
        .await;

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": to_email, "password": password }))
        .await;

    // Verificarlos
    verify_user(&ctx.pool, from_email).await;
    verify_user(&ctx.pool, to_email).await;

    // Crear subject/group y una sesión
    sqlx::query!(
        "INSERT INTO subject_groups (subject, grp) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        "MAT202",
        "B"
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    let starts_at: DateTime<Utc> = DateTime::from_str("2026-05-19T10:00:00Z").unwrap();
    sqlx::query!(
        "INSERT INTO sessions (subject, grp, starts_at, duration_min) VALUES ($1, $2, $3, $4)",
        "MAT202",
        "B",
        starts_at,
        90i32
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    // Suscribir "from" al grupo
    let row = sqlx::query!("SELECT id FROM users WHERE email = $1", from_email)
        .fetch_one(&ctx.pool)
        .await
        .unwrap();

    let from_user_id = row.id;

    sqlx::query!(
        "INSERT INTO schedule (user_id, subject, grp) VALUES ($1, $2, $3)",
        from_user_id,
        "MAT202",
        "B"
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    // Login del usuario destino
    let token = login_user(&ctx.server, to_email, password).await;

    let response = ctx
        .server
        .post(&format!("/schedule/copy?user={from_email}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["message"], "Copy was successful");
    assert_eq!(body["copied_count"], 1);

    // Comprobar que el usuario destino ahora tiene una entrada en schedule
    let row = sqlx::query!("SELECT id FROM users WHERE email = $1", to_email)
        .fetch_one(&ctx.pool)
        .await
        .unwrap();
    let to_user_id = row.id;

    let schedules = sqlx::query!(
        "SELECT subject, grp FROM schedule WHERE user_id = $1",
        to_user_id
    )
    .fetch_all(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(schedules.len(), 1);
    assert_eq!(schedules[0].subject, "MAT202");
    assert_eq!(schedules[0].grp, "B");
}

#[tokio::test]
async fn copy_schedule_devuelve_400_si_from_user_es_to_user() {
    let ctx = setup().await;

    let email = "usuario@uniovi.es";
    let password = "Password123";
    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": email, "password": password }))
        .await;
    verify_user(&ctx.pool, email).await;
    let token = login_user(&ctx.server, email, password).await;

    let response = ctx
        .server
        .post(&format!("/schedule/copy?user={email}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_bad_request();
}
