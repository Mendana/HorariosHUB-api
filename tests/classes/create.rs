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

fn base_payload() -> serde_json::Value {
    json!({
        "subject": "ALG",
        "subjectType": "Teoría",
        "classroom": "Aula 1",
        "startTime": "2025-09-15T09:00:00Z",
        "endTime":   "2025-09-15T10:30:00Z"
    })
}

#[tokio::test]
async fn post_classes_devuelve_201_como_profesor() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&base_payload())
        .await;

    response.assert_status(StatusCode::CREATED);

    let body: serde_json::Value = response.json();
    assert_eq!(body["subject"], "ALG");
    assert_eq!(body["subjectType"], "Teoría");
    assert_eq!(body["classroom"], "Aula 1");
    assert!(body["id"].is_string());
    assert!(body["startTime"].is_string());
    assert!(body["endTime"].is_string());
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
            "subject": "CAL",
            "subjectType": "Práctica",
            "startTime": "2025-09-16T11:00:00Z",
            "endTime":   "2025-09-16T12:00:00Z"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    assert_eq!(body["subject"], "CAL");
}

#[tokio::test]
async fn post_classes_devuelve_403_como_student() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student@uniovi.es", "student").await;

    let response = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&base_payload())
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn post_classes_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx.server.post("/classes").json(&base_payload()).await;

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
            "subject": "ALG",
            "subjectType": "Teoría",
            "startTime": "2025-09-15T09:00:00Z",
            "endTime":   "2025-09-15T10:30:00Z"
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
            "subject": "ALG", "subjectType": "Teoría",
            "startTime": "2025-09-15T08:00:00Z",
            "endTime":   "2025-09-15T08:30:00Z"
        }))
        .await;
    let body: serde_json::Value = r.json();
    let start: chrono::DateTime<chrono::Utc> = body["startTime"].as_str().unwrap().parse().unwrap();
    let end: chrono::DateTime<chrono::Utc> = body["endTime"].as_str().unwrap().parse().unwrap();
    assert_eq!((end - start).num_minutes(), 30);

    // 120 min
    let r = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "subject": "ALG", "subjectType": "Teoría",
            "startTime": "2025-09-15T10:00:00Z",
            "endTime":   "2025-09-15T12:00:00Z"
        }))
        .await;
    let body: serde_json::Value = r.json();
    let start: chrono::DateTime<chrono::Utc> = body["startTime"].as_str().unwrap().parse().unwrap();
    let end: chrono::DateTime<chrono::Utc> = body["endTime"].as_str().unwrap().parse().unwrap();
    assert_eq!((end - start).num_minutes(), 120);
}

#[tokio::test]
async fn post_classes_devuelve_400_si_duracion_no_es_multiplo_de_30() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof4@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "subject": "ALG",
            "subjectType": "Teoría",
            "startTime": "2025-09-15T09:00:00Z",
            "endTime":   "2025-09-15T09:45:00Z"   // 45 min — no múltiplo de 30
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_classes_devuelve_400_si_end_time_antes_de_start_time() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof5@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "subject": "ALG",
            "subjectType": "Teoría",
            "startTime": "2025-09-15T10:00:00Z",
            "endTime":   "2025-09-15T09:00:00Z"   // end antes que start
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_classes_crea_subject_group_si_no_existe() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof6@uniovi.es", "professor").await;

    ctx.server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "subject": "NUEVA_ASIG",
            "subjectType": "Laboratorio",
            "startTime": "2025-09-15T09:00:00Z",
            "endTime":   "2025-09-15T10:30:00Z"
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
    let token = login_as(&ctx, "prof7@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&base_payload())
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
    let token = login_as(&ctx, "prof8@uniovi.es", "professor").await;

    ctx.server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&base_payload())
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

#[tokio::test]
async fn post_classes_examen_notifica_a_suscriptores() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof9@uniovi.es", "professor").await;

    let _student_token = login_as(&ctx, "stu9@uniovi.es", "student").await;
    let student_id: uuid::Uuid =
        sqlx::query_scalar!("SELECT id FROM users WHERE email = $1", "stu9@uniovi.es")
            .fetch_one(&ctx.pool)
            .await
            .unwrap();

    sqlx::query!(
        "INSERT INTO subject_groups (subject, grp) VALUES ('MAT', 'Examen') ON CONFLICT DO NOTHING"
    )
    .execute(&ctx.pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO schedule (user_id, subject, grp) VALUES ($1, 'MAT', 'Examen')",
        student_id
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    let response = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "subject": "MAT",
            "subjectType": "Examen",
            "startTime": "2025-10-01T09:00:00Z",
            "endTime":   "2025-10-01T11:00:00Z"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);

    let notification = sqlx::query!(
        r#"SELECT type::text AS notif_type FROM notifications WHERE user_id = $1"#,
        student_id
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("Debe existir una notificación de examen para el suscriptor");

    assert_eq!(notification.notif_type.as_deref(), Some("exam_added"));
}

#[tokio::test]
async fn post_classes_sin_ser_examen_no_notifica() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof10@uniovi.es", "professor").await;

    let _student_token = login_as(&ctx, "stu10@uniovi.es", "student").await;
    let student_id: uuid::Uuid =
        sqlx::query_scalar!("SELECT id FROM users WHERE email = $1", "stu10@uniovi.es")
            .fetch_one(&ctx.pool)
            .await
            .unwrap();

    sqlx::query!(
        "INSERT INTO subject_groups (subject, grp) VALUES ('ALG', 'Teoría') ON CONFLICT DO NOTHING"
    )
    .execute(&ctx.pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO schedule (user_id, subject, grp) VALUES ($1, 'ALG', 'Teoría')",
        student_id
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    ctx.server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&base_payload())
        .await;

    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1",
        student_id
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(count, Some(0));
}
