// tests/classes_delete.rs
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

    // Primera vez — ok
    ctx.server
        .delete(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    // Segunda vez — ya no existe
    let response = ctx
        .server
        .delete(&format!("/classes/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}
