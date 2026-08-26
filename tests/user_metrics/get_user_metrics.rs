use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde_json::json;

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
async fn get_user_metrics_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx.server.get("/user-metrics").await;

    response.assert_status_unauthorized();
}

#[tokio::test]
async fn get_user_metrics_devuelve_400_con_semester_invalido() {
    let ctx = setup().await;
    let token = login_as(&ctx, "metrics_invalid@uniovi.es", "student").await;

    let response = ctx
        .server
        .get("/user-metrics?semester=3")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_bad_request();
}

#[tokio::test]
async fn get_user_metrics_sin_sesiones_devuelve_todo_a_cero() {
    let ctx = setup().await;
    let token = login_as(&ctx, "metrics_empty@uniovi.es", "student").await;

    let response = ctx
        .server
        .get("/user-metrics")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["total_hours"], 0.0);
    assert_eq!(body["weekly_average_hours"], 0.0);
    assert!(body["by_weekday"].as_array().unwrap().is_empty());
    assert_eq!(body["days_without_class"], json!([1, 2, 3, 4, 5]));
    assert!(body["by_type"].as_array().unwrap().is_empty());
    assert!(body["by_subject"].as_array().unwrap().is_empty());
    assert!(body["earliest_start_time"].is_null());
    assert!(body["busiest_week"].is_null());
    assert_eq!(body["completed_classes"], 0);
    assert_eq!(body["remaining_classes"], 0);
    assert!(body["semesters"].is_null());
}

#[tokio::test]
async fn get_user_metrics_devuelve_desglose_completo_con_sesiones_en_ambos_semestres() {
    let ctx = setup().await;
    let email = "metrics_full@uniovi.es";
    let token = login_as(&ctx, email, "student").await;

    // Semestre 1 (septiembre) — lunes, teoría, 1.5h — ya pasada respecto al "hoy" de los tests (2026-07-20)... en realidad futura.
    let s1: DateTime<Utc> = DateTime::from_str("2026-09-07T09:00:00Z").unwrap();
    seed_session(&ctx.pool, "MAT", "T.1", s1, 90).await;
    subscribe(&ctx.pool, email, "MAT", "T.1").await;

    // Semestre 2 (febrero) — lunes, laboratorio, 2h — ya pasada.
    let s2: DateTime<Utc> = DateTime::from_str("2026-02-02T11:00:00Z").unwrap();
    seed_session(&ctx.pool, "FIS", "PL.1", s2, 120).await;
    subscribe(&ctx.pool, email, "FIS", "PL.1").await;

    let response = ctx
        .server
        .get("/user-metrics")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();

    assert_eq!(body["total_hours"], 3.5);
    assert_eq!(body["completed_classes"], 1);
    assert_eq!(body["remaining_classes"], 1);

    let by_subject = body["by_subject"].as_array().unwrap();
    assert_eq!(by_subject.len(), 2);

    let by_type = body["by_type"].as_array().unwrap();
    assert_eq!(by_type.len(), 2);
    assert!(
        by_type
            .iter()
            .any(|t| t["session_type"] == "teoria" && t["hours"] == 1.5)
    );
    assert!(
        by_type
            .iter()
            .any(|t| t["session_type"] == "laboratorio" && t["hours"] == 2.0)
    );

    let semesters = &body["semesters"];
    assert!(!semesters.is_null());
    assert_eq!(semesters["semester_1"]["total_hours"], 1.5);
    assert_eq!(semesters["semester_2"]["total_hours"], 2.0);
}

#[tokio::test]
async fn get_user_metrics_filtra_por_semester() {
    let ctx = setup().await;
    let email = "metrics_filter@uniovi.es";
    let token = login_as(&ctx, email, "student").await;

    let s1: DateTime<Utc> = DateTime::from_str("2026-09-07T09:00:00Z").unwrap();
    seed_session(&ctx.pool, "MAT", "T.1", s1, 90).await;
    subscribe(&ctx.pool, email, "MAT", "T.1").await;

    let s2: DateTime<Utc> = DateTime::from_str("2026-02-02T11:00:00Z").unwrap();
    seed_session(&ctx.pool, "FIS", "PL.1", s2, 120).await;
    subscribe(&ctx.pool, email, "FIS", "PL.1").await;

    let response = ctx
        .server
        .get("/user-metrics?semester=1")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["total_hours"], 1.5);
}
