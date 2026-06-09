use crate::common::{login_user, setup};
use axum::http::StatusCode;
use serde_json::json;

async fn login_as(ctx: &crate::common::TestContext, email: &str, role: &str) -> String {
    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": email, "password": "Password123" }))
        .await;

    sqlx::query("UPDATE users SET role = $1::user_role, verified = true WHERE email = $2")
        .bind(role)
        .bind(email)
        .execute(&ctx.pool)
        .await
        .unwrap();

    login_user(&ctx.server, email, "Password123").await
}

#[tokio::test]
async fn post_classes_devuelve_201_como_profesor() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "name": "ALG",
            "type": "Teoría",
            "classroom": "Aula 1",
            "date": { "year": 2025, "month": 9, "day": 15 },
            "startTime": "09:00",
            "durationMinutes": 90
        }))
        .await;

    response.assert_status(StatusCode::CREATED);

    let body: serde_json::Value = response.json();
    assert_eq!(body["name"], "ALG");
    assert_eq!(body["type"], "Teoría");
    assert_eq!(body["endTime"], "10:30"); // 09:00 + 90 min
    assert_eq!(body["classroom"], "Aula 1");
    assert!(body["id"].is_string());
}

#[tokio::test]
async fn post_classes_devuelve_201_como_admin() {
    let ctx = setup().await;
    let token = login_as(&ctx, "admin@uniovi.es", "admin").await;

    let response = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "name": "CAL",
            "type": "Práctica",
            "date": { "year": 2025, "month": 9, "day": 16 },
            "startTime": "11:00",
            "durationMinutes": 60
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    assert_eq!(body["endTime"], "12:00");
}

#[tokio::test]
async fn post_classes_devuelve_403_como_student() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student@uniovi.es", "student").await;

    let response = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "name": "ALG",
            "type": "Teoría",
            "date": { "year": 2025, "month": 9, "day": 15 },
            "startTime": "09:00",
            "durationMinutes": 90
        }))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn post_classes_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/classes")
        .json(&json!({
            "name": "ALG",
            "type": "Teoría",
            "date": { "year": 2025, "month": 9, "day": 15 },
            "startTime": "09:00",
            "durationMinutes": 90
        }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn post_classes_sin_classroom_es_valido() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof2@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "name": "ALG",
            "type": "Teoría",
            // sin classroom — es opcional
            "date": { "year": 2025, "month": 9, "day": 15 },
            "startTime": "09:00",
            "durationMinutes": 90
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    assert!(body["classroom"].is_null());
}

#[tokio::test]
async fn post_classes_calcula_end_time_correctamente() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof3@uniovi.es", "professor").await;

    // 30 min
    let r = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "name": "ALG", "type": "Teoría",
            "date": { "year": 2025, "month": 9, "day": 15 },
            "startTime": "08:00", "durationMinutes": 30
        }))
        .await;
    assert_eq!(r.json::<serde_json::Value>()["endTime"], "08:30");

    // 120 min
    let r = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "name": "ALG", "type": "Teoría",
            "date": { "year": 2025, "month": 9, "day": 15 },
            "startTime": "10:00", "durationMinutes": 120
        }))
        .await;
    assert_eq!(r.json::<serde_json::Value>()["endTime"], "12:00");
}

#[tokio::test]
async fn post_classes_devuelve_422_si_duracion_no_es_multiplo_de_30() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof4@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "name": "ALG",
            "type": "Teoría",
            "date": { "year": 2025, "month": 9, "day": 15 },
            "startTime": "09:00",
            "durationMinutes": 45
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_classes_devuelve_400_si_fecha_invalida() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof5@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "name": "ALG",
            "type": "Teoría",
            "date": { "year": 2025, "month": 13, "day": 1 }, // mes 13 no existe
            "startTime": "09:00",
            "durationMinutes": 90
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_classes_devuelve_400_si_hora_invalida() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof6@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "name": "ALG",
            "type": "Teoría",
            "date": { "year": 2025, "month": 9, "day": 15 },
            "startTime": "25:00",   // hora inválida
            "durationMinutes": 90
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_classes_crea_subject_group_si_no_existe() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof7@uniovi.es", "professor").await;

    ctx.server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "name": "NUEVA_ASIG",
            "type": "Laboratorio",
            "date": { "year": 2025, "month": 9, "day": 15 },
            "startTime": "09:00",
            "durationMinutes": 90
        }))
        .await;

    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM subject_groups WHERE subject = $1 AND grp = $2",
        "NUEVA_ASIG",
        "Laboratorio"
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(exists, Some(1));
}

#[tokio::test]
async fn post_classes_sesion_tiene_source_manual() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof8@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "name": "ALG",
            "type": "Teoría",
            "date": { "year": 2025, "month": 9, "day": 15 },
            "startTime": "09:00",
            "durationMinutes": 90
        }))
        .await;

    let body: serde_json::Value = response.json();
    let session_id: uuid::Uuid = body["id"].as_str().unwrap().parse().unwrap();

    let session = sqlx::query!(
        "SELECT source::text, scraped_at, created_by FROM sessions WHERE id = $1",
        session_id
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(session.source, Some("manual".to_string()));
    assert!(session.scraped_at.is_none());
    assert!(session.created_by.is_some());
}

#[tokio::test]
async fn post_classes_registra_change_aprobado() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof9@uniovi.es", "professor").await;

    ctx.server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "name": "ALG",
            "type": "Teoría",
            "classroom": "Aula 1",
            "date": { "year": 2025, "month": 9, "day": 15 },
            "startTime": "09:00",
            "durationMinutes": 90
        }))
        .await;

    let row = sqlx::query!(
        r#"
        SELECT change_type::text AS change_type, change_status::text AS change_status,
               session_id, subject, grp, new_duration, new_classroom
        FROM changes
        WHERE change_type = 'create' AND change_status = 'approved'
          AND subject = 'ALG' AND grp = 'Teoría'
        "#
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("Debe existir un change de tipo create aprobado");

    assert_eq!(row.change_type.as_deref(), Some("create"));
    assert_eq!(row.change_status.as_deref(), Some("approved"));
    assert!(
        row.session_id.is_none(),
        "chk_create exige session_id IS NULL"
    );
    assert_eq!(row.subject.as_deref(), Some("ALG"));
    assert_eq!(row.grp.as_deref(), Some("Teoría"));
    assert_eq!(row.new_duration, Some(90));
    assert_eq!(row.new_classroom.as_deref(), Some("Aula 1"));
}
