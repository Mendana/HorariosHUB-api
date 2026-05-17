use crate::common::{login_user, setup};
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn get_auth_me_devuelve_200_con_usuario_autenticado() {
    let ctx = setup().await;

    let email = "me@uniovi.es";
    let password = "Password123";
    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": email, "password": password }))
        .await;

    let token = login_user(&ctx.server, email, password).await;

    let response = ctx
        .server
        .get("/auth/me")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["email"], email);
    assert_eq!(body["role"], "student");
}

#[tokio::test]
async fn get_auth_me_devuelve_401_sin_token() {
    let ctx = setup().await;

    let response = ctx.server.get("/auth/me").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_auth_me_devuelve_401_con_token_invalido() {
    let ctx = setup().await;

    let response = ctx
        .server
        .get("/auth/me")
        .add_header("Authorization", "Bearer invalid_token_xyz")
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_auth_me_devuelve_401_con_bearer_invalido() {
    let ctx = setup().await;

    let response = ctx
        .server
        .get("/auth/me")
        .add_header("Authorization", "valid_jwt_token_would_go_here")
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_auth_me_devuelve_200_con_datos_correctos() {
    let ctx = setup().await;

    let email = "verify@uniovi.es";
    let password = "Password123";
    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": email, "password": password }))
        .await;

    let token = login_user(&ctx.server, email, password).await;

    let response = ctx
        .server
        .get("/auth/me")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert!(body.is_object());
    assert_eq!(body["email"], email);
    assert_eq!(body["role"], "student");
    assert!(!body["role"].is_null());
    assert!(!body["email"].is_null());
}

#[tokio::test]
async fn get_auth_me_devuelve_token_expiration_error() {
    let ctx = setup().await;

    let expired_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwiZW1haWwiOiJ0ZXN0QHVuaW92aS5lcyIsInJvbGUiOiJzdHVkZW50IiwiZXhwIjoxNjAwMDAwMDAwLCJpYXQiOjE2MDAwMDAwMDB9.invalid_signature";

    let response = ctx
        .server
        .get("/auth/me")
        .add_header("Authorization", format!("Bearer {expired_token}"))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_auth_me_con_multiples_usuarios() {
    let ctx = setup().await;

    let email1 = "user1@uniovi.es";
    let email2 = "user2@uniovi.es";
    let password = "Password123";

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": email1, "password": password }))
        .await;

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": email2, "password": password }))
        .await;

    let token1 = login_user(&ctx.server, email1, password).await;
    let token2 = login_user(&ctx.server, email2, password).await;

    let response1 = ctx
        .server
        .get("/auth/me")
        .add_header("Authorization", format!("Bearer {token1}"))
        .await;
    response1.assert_status_ok();
    assert_eq!(response1.json::<serde_json::Value>()["email"], email1);

    let response2 = ctx
        .server
        .get("/auth/me")
        .add_header("Authorization", format!("Bearer {token2}"))
        .await;
    response2.assert_status_ok();
    assert_eq!(response2.json::<serde_json::Value>()["email"], email2);
}
