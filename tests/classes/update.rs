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

async fn create_session(ctx: &crate::common::TestContext, token: &str) -> String {
    let r = ctx
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
    assert_eq!(body["startTime"], "09:00");
    assert_eq!(body["endTime"], "10:30");
}

#[tokio::test]
async fn patch_classes_actualiza_hora_y_recalcula_end_time() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof2@uniovi.es", "professor").await;
    let id = create_session(&ctx, &token).await;

    let response = ctx
        .server
        .patch(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "startTime": "11:00" }))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["startTime"], "11:00");
    assert_eq!(body["endTime"], "12:30"); // 11:00 + 90 min originales
}

#[tokio::test]
async fn patch_classes_actualiza_duracion_y_recalcula_end_time() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof3@uniovi.es", "professor").await;
    let id = create_session(&ctx, &token).await;

    let response = ctx
        .server
        .patch(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "durationMinutes": 60 }))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["startTime"], "09:00");
    assert_eq!(body["endTime"], "10:00"); // 09:00 + 60 min nuevos
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
async fn patch_classes_devuelve_422_si_duracion_no_multiplo_30() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof7@uniovi.es", "professor").await;
    let id = create_session(&ctx, &token).await;

    let response = ctx
        .server
        .patch(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "durationMinutes": 45 }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}
