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

/// Crea una sesión de 90 min: 2025-09-15 09:00 → 10:30 UTC
async fn create_session(ctx: &crate::common::TestContext, token: &str) -> String {
    let r = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "subject": "ALG",
            "subjectType": "Teoría",
            "classroom": "Aula 1",
            "startTime": "2025-09-15T09:00:00Z",
            "endTime":   "2025-09-15T10:30:00Z"
        }))
        .await;
    r.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn patch_classes_actualiza_classroom() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof@uniovi.es", "professor").await;
    let id = create_session(&ctx, &token).await;

    let response = ctx
        .server
        .patch(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "classroom": "Aula 2" }))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["classroom"], "Aula 2");

    // Hora no cambia
    let start: chrono::DateTime<chrono::Utc> = body["startTime"].as_str().unwrap().parse().unwrap();
    let end: chrono::DateTime<chrono::Utc> = body["endTime"].as_str().unwrap().parse().unwrap();
    assert_eq!((end - start).num_minutes(), 90);
}

#[tokio::test]
async fn patch_classes_actualiza_hora_y_recalcula_end_time() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof2@uniovi.es", "professor").await;
    let id = create_session(&ctx, &token).await;

    // Solo cambia startTime → endTime = new_start + duración original (90 min)
    let response = ctx
        .server
        .patch(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "startTime": "2025-09-15T11:00:00Z" }))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let start: chrono::DateTime<chrono::Utc> = body["startTime"].as_str().unwrap().parse().unwrap();
    let end: chrono::DateTime<chrono::Utc> = body["endTime"].as_str().unwrap().parse().unwrap();
    assert_eq!(start.format("%H:%M").to_string(), "11:00");
    assert_eq!((end - start).num_minutes(), 90); // duración original conservada
}

#[tokio::test]
async fn patch_classes_actualiza_duracion_y_recalcula_end_time() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof3@uniovi.es", "professor").await;
    let id = create_session(&ctx, &token).await;

    // Solo cambia endTime → nueva duración = end - existing_start
    let response = ctx
        .server
        .patch(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "endTime": "2025-09-15T10:00:00Z" }))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let start: chrono::DateTime<chrono::Utc> = body["startTime"].as_str().unwrap().parse().unwrap();
    let end: chrono::DateTime<chrono::Utc> = body["endTime"].as_str().unwrap().parse().unwrap();
    assert_eq!(start.format("%H:%M").to_string(), "09:00");
    assert_eq!((end - start).num_minutes(), 60); // 90 → 60
}

#[tokio::test]
async fn patch_classes_marca_is_overridden() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof4@uniovi.es", "professor").await;
    let id = create_session(&ctx, &token).await;

    ctx.server
        .patch(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "classroom": "Aula 3" }))
        .await;

    let overridden = sqlx::query_scalar!(
        "SELECT is_overridden FROM sessions WHERE id = $1",
        uuid::Uuid::parse_str(&id).unwrap()
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert!(overridden);
}

#[tokio::test]
async fn patch_classes_devuelve_404_si_no_existe() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof5@uniovi.es", "professor").await;

    let response = ctx
        .server
        .patch(&format!("/classes/{}", uuid::Uuid::new_v4()))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "classroom": "Aula 2" }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn patch_classes_devuelve_403_como_student() {
    let ctx = setup().await;
    let prof_token = login_as(&ctx, "prof6@uniovi.es", "professor").await;
    let id = create_session(&ctx, &prof_token).await;

    let student_token = login_as(&ctx, "stu@uniovi.es", "student").await;

    let response = ctx
        .server
        .patch(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {student_token}"))
        .json(&json!({ "classroom": "Aula 2" }))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn patch_classes_devuelve_400_si_duracion_no_multiplo_30() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof7@uniovi.es", "professor").await;
    let id = create_session(&ctx, &token).await;

    // endTime genera 45 min (no múltiplo de 30)
    let response = ctx
        .server
        .patch(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "endTime": "2025-09-15T09:45:00Z" }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn patch_classes_registra_change_aprobado() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof8@uniovi.es", "professor").await;
    let id = create_session(&ctx, &token).await;
    let session_uuid = uuid::Uuid::parse_str(&id).unwrap();

    // Cambia duración (90 → 60) y classroom
    ctx.server
        .patch(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "endTime":   "2025-09-15T10:00:00Z",
            "classroom": "Aula 9"
        }))
        .await;

    let row = sqlx::query!(
        r#"
        SELECT change_type::text AS change_type, change_status::text AS change_status,
               session_id, prev_duration, prev_classroom, new_duration, new_classroom
        FROM changes
        WHERE change_type = 'modify' AND change_status = 'approved'
          AND session_id = $1
        "#,
        session_uuid
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("Debe existir un change de tipo modify aprobado");

    assert_eq!(row.change_type.as_deref(), Some("modify"));
    assert_eq!(row.change_status.as_deref(), Some("approved"));
    assert_eq!(row.session_id, Some(session_uuid));
    assert_eq!(row.prev_duration, Some(90));
    assert_eq!(row.prev_classroom.as_deref(), Some("Aula 1"));
    assert_eq!(row.new_duration, Some(60));
    assert_eq!(row.new_classroom.as_deref(), Some("Aula 9"));
}

#[tokio::test]
async fn patch_classes_notifica_a_suscriptores() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof9@uniovi.es", "professor").await;
    let id = create_session(&ctx, &token).await;
    let session_uuid = uuid::Uuid::parse_str(&id).unwrap();

    let _student_token = login_as(&ctx, "stu9@uniovi.es", "student").await;
    let student_id: uuid::Uuid =
        sqlx::query_scalar!("SELECT id FROM users WHERE email = $1", "stu9@uniovi.es")
            .fetch_one(&ctx.pool)
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
        .patch(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "classroom": "Aula 2" }))
        .await;

    let notification = sqlx::query!(
        r#"SELECT type::text AS notif_type, session_id FROM notifications WHERE user_id = $1"#,
        student_id
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("Debe existir una notificación para el suscriptor");

    assert_eq!(notification.notif_type.as_deref(), Some("session_modified"));
    assert_eq!(notification.session_id, Some(session_uuid));
}
