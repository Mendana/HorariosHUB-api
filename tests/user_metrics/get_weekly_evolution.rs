use std::str::FromStr;

use chrono::{DateTime, Utc};

use crate::common::{login_as, setup};

async fn seed_session(
    pool: &sqlx::PgPool,
    subject: &str,
    grp: &str,
    starts_at: DateTime<Utc>,
    duration_min: i32,
) {
    sqlx::query!(
        "INSERT INTO subject_groups (subject, grp) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        subject,
        grp
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO sessions (subject, grp, starts_at, duration_min) VALUES ($1, $2, $3, $4)",
        subject,
        grp,
        starts_at,
        duration_min
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn subscribe(pool: &sqlx::PgPool, email: &str, subject: &str, grp: &str) {
    let row = sqlx::query!("SELECT id FROM users WHERE email = $1", email)
        .fetch_one(pool)
        .await
        .unwrap();

    sqlx::query!(
        "INSERT INTO schedule (user_id, subject, grp) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        row.id,
        subject,
        grp
    )
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn get_weekly_evolution_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx.server.get("/user-metrics/weekly").await;

    response.assert_status_unauthorized();
}

#[tokio::test]
async fn get_weekly_evolution_devuelve_400_con_semester_invalido() {
    let ctx = setup().await;
    let token = login_as(&ctx, "weekly_invalid@uniovi.es", "student").await;

    let response = ctx
        .server
        .get("/user-metrics/weekly?semester=3")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_bad_request();
}

#[tokio::test]
async fn get_weekly_evolution_sin_sesiones_devuelve_array_vacio() {
    let ctx = setup().await;
    let token = login_as(&ctx, "weekly_empty@uniovi.es", "student").await;

    let response = ctx
        .server
        .get("/user-metrics/weekly")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn get_weekly_evolution_rellena_huecos_y_va_ordenada_ascendente() {
    let ctx = setup().await;
    let email = "weekly_gaps@uniovi.es";
    let token = login_as(&ctx, email, "student").await;

    // Semana ISO 37 de 2026
    let first: DateTime<Utc> = DateTime::from_str("2026-09-07T09:00:00Z").unwrap();
    seed_session(&ctx.pool, "MAT", "T.1", first, 60).await;
    subscribe(&ctx.pool, email, "MAT", "T.1").await;

    // Semana ISO 40 de 2026 (deja 38 y 39 vacías en medio)
    let last: DateTime<Utc> = DateTime::from_str("2026-09-28T09:00:00Z").unwrap();
    seed_session(&ctx.pool, "MAT", "T.1", last, 120).await;
    subscribe(&ctx.pool, email, "MAT", "T.1").await;

    let response = ctx
        .server
        .get("/user-metrics/weekly")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let entries = body.as_array().unwrap();

    assert_eq!(entries.len(), 4);
    let weeks: Vec<i64> = entries
        .iter()
        .map(|e| e["iso_week"].as_i64().unwrap())
        .collect();
    assert_eq!(weeks, vec![37, 38, 39, 40]);

    assert_eq!(entries[0]["total_hours"], 1.0);
    assert_eq!(entries[0]["class_count"], 1);

    // Semanas intermedias sin clase: van a cero, no desaparecen.
    assert_eq!(entries[1]["total_hours"], 0.0);
    assert_eq!(entries[1]["class_count"], 0);
    assert_eq!(entries[2]["total_hours"], 0.0);

    assert_eq!(entries[3]["total_hours"], 2.0);
    assert_eq!(entries[3]["class_count"], 1);
}

#[tokio::test]
async fn get_weekly_evolution_filtra_por_semester() {
    let ctx = setup().await;
    let email = "weekly_filter@uniovi.es";
    let token = login_as(&ctx, email, "student").await;

    let s1: DateTime<Utc> = DateTime::from_str("2026-09-07T09:00:00Z").unwrap();
    seed_session(&ctx.pool, "MAT", "T.1", s1, 60).await;
    subscribe(&ctx.pool, email, "MAT", "T.1").await;

    let s2: DateTime<Utc> = DateTime::from_str("2026-02-02T09:00:00Z").unwrap();
    seed_session(&ctx.pool, "FIS", "PL.1", s2, 120).await;
    subscribe(&ctx.pool, email, "FIS", "PL.1").await;

    let response = ctx
        .server
        .get("/user-metrics/weekly?semester=1")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let entries = body.as_array().unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["iso_week"], 37);
}
