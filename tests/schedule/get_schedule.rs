use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde_json::json;

use crate::common::{setup, verify_user};

#[tokio::test]
async fn get_schedule_devuelve_404_si_identificador_no_existe() {
    let ctx = setup().await;

    let response = ctx
        .server
        .get("/schedule/noexiste?start=2026-05-19T00:00:00Z")
        .await;

    response.assert_status_not_found();
}

#[tokio::test]
async fn get_schedule_devuelve_400_si_identificador_no_valido() {
    let ctx = setup().await;

    // Too short identifier! (< 6 characters)
    let response = ctx
        .server
        .get("/schedule/short?start=2026-05-19T00:00:00Z")
        .await;

    response.assert_status_bad_request();
}

#[tokio::test]
async fn get_schedule_devuelve_200_con_array_vacio_si_identificador_correcto_pero_sin_datos() {
    let ctx = setup().await;

    let email = "usuario@uniovi.es";
    let password = "Password123";
    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": email, "password": password }))
        .await;
    verify_user(&ctx.pool, email).await;

    let response = ctx
        .server
        .get("/schedule/usuario@uniovi.es?start=2026-05-19T00:00:00Z")
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body["sessions"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn get_schedule_devuelve_200_con_array_con_datos_si_todo_bien() {
    let ctx = setup().await;

    let email = "data@uniovi.es";
    let password = "Password123";
    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": email, "password": password }))
        .await;

    // Marcar usuario como verificado
    verify_user(&ctx.pool, email).await;

    // Crear subject/group y una sesión dentro de la semana
    sqlx::query!(
        "INSERT INTO subject_groups (subject, grp) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        "MAT101",
        "A"
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    let starts_at: DateTime<Utc> = DateTime::from_str("2026-05-19T10:00:00Z").unwrap();
    sqlx::query!(
        "INSERT INTO sessions (subject, grp, starts_at, duration_min) VALUES ($1, $2, $3, $4)",
        "MAT101",
        "A",
        starts_at,
        90i32
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    // Suscribir usuario al grupo
    let row = sqlx::query!("SELECT id FROM users WHERE email = $1", email)
        .fetch_one(&ctx.pool)
        .await
        .unwrap();

    let user_id = row.id;

    sqlx::query!(
        "INSERT INTO schedule (user_id, subject, grp) VALUES ($1, $2, $3)",
        user_id,
        "MAT101",
        "A"
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    let response = ctx
        .server
        .get(&format!("/schedule/{email}?start=2026-05-19T00:00:00Z"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let sessions = body["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0]["subject"], "MAT101");
    assert_eq!(sessions[0]["group"], "A");
}
