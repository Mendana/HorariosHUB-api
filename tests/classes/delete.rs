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

/// Crea una sesión de 90 min: 2025-09-15 09:00 → 10:30 UTC con classroom "Aula 1"
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
async fn delete_classes_devuelve_200_como_profesor() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof@uniovi.es", "professor").await;
    let id = create_session(&ctx, &token).await;

    let response = ctx
        .server
        .delete(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["message"], "Clase eliminada");
}

#[tokio::test]
async fn delete_classes_devuelve_200_como_admin() {
    let ctx = setup().await;
    let token = login_as(&ctx, "admin@uniovi.es", "admin").await;
    let id = create_session(&ctx, &token).await;

    let response = ctx
        .server
        .delete(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
}

#[tokio::test]
async fn delete_classes_la_sesion_no_existe_en_bbdd_tras_borrar() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof2@uniovi.es", "professor").await;
    let id = create_session(&ctx, &token).await;

    ctx.server
        .delete(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM sessions WHERE id = $1",
        uuid::Uuid::parse_str(&id).unwrap()
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(count, Some(0));
}

#[tokio::test]
async fn delete_classes_devuelve_404_si_no_existe() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof3@uniovi.es", "professor").await;

    let response = ctx
        .server
        .delete(&format!("/classes/{}", uuid::Uuid::new_v4()))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_classes_devuelve_403_como_student() {
    let ctx = setup().await;
    let prof_token = login_as(&ctx, "prof4@uniovi.es", "professor").await;
    let id = create_session(&ctx, &prof_token).await;

    let student_token = login_as(&ctx, "stu@uniovi.es", "student").await;

    let response = ctx
        .server
        .delete(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {student_token}"))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn delete_classes_devuelve_401_sin_autenticar() {
    let ctx = setup().await;
    let prof_token = login_as(&ctx, "prof5@uniovi.es", "professor").await;
    let id = create_session(&ctx, &prof_token).await;

    let response = ctx.server.delete(&format!("/classes/{id}")).await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn delete_classes_devuelve_404_en_segundo_borrado() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof6@uniovi.es", "professor").await;
    let id = create_session(&ctx, &token).await;

    ctx.server
        .delete(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    let response = ctx
        .server
        .delete(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_classes_registra_change_aprobado() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof7@uniovi.es", "professor").await;
    let id = create_session(&ctx, &token).await;

    ctx.server
        .delete(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    // ON DELETE SET NULL pone session_id a NULL en el change
    let row = sqlx::query!(
        r#"
        SELECT change_type::text AS change_type, change_status::text AS change_status,
               session_id, prev_duration, prev_classroom
        FROM changes
        WHERE change_type = 'delete' AND change_status = 'approved'
          AND prev_duration = 90
        "#
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("Debe existir un change de tipo delete aprobado");

    assert_eq!(row.change_type.as_deref(), Some("delete"));
    assert_eq!(row.change_status.as_deref(), Some("approved"));
    assert!(
        row.session_id.is_none(),
        "ON DELETE SET NULL debe haber puesto session_id a NULL"
    );
    assert_eq!(row.prev_duration, Some(90));
    assert_eq!(row.prev_classroom.as_deref(), Some("Aula 1"));
}

#[tokio::test]
async fn delete_classes_notifica_a_suscriptores() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof8@uniovi.es", "professor").await;
    let id = create_session(&ctx, &token).await;

    let _student_token = login_as(&ctx, "stu2@uniovi.es", "student").await;
    let student_id: uuid::Uuid =
        sqlx::query_scalar!("SELECT id FROM users WHERE email = $1", "stu2@uniovi.es")
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
        .delete(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    let notification = sqlx::query!(
        r#"SELECT type::text AS notif_type, session_id FROM notifications WHERE user_id = $1"#,
        student_id
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("Debe existir una notificación para el suscriptor");

    assert_eq!(notification.notif_type.as_deref(), Some("session_deleted"));
    // ON DELETE SET NULL también aplica a notifications.session_id
    assert!(notification.session_id.is_none());
}
