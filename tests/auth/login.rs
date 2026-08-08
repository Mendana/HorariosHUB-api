use crate::common::{setup, verify_user};
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn post_login_devuelve_200_y_cookie() {
    let ctx = setup().await;

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": "login@uniovi.es", "password": "Password123" }))
        .await;
    verify_user(&ctx.pool, "login@uniovi.es").await;

    let response = ctx
        .server
        .post("/auth/login")
        .json(&json!({ "email": "login@uniovi.es", "password": "Password123" }))
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["user"]["email"], "login@uniovi.es");
    assert_eq!(body["user"]["role"], "student");

    let cookie_header = response.headers().get("set-cookie");
    assert!(cookie_header.is_some());
    let cookie = cookie_header.unwrap().to_str().unwrap();
    assert!(cookie.contains("access_token="));
    assert!(cookie.contains("HttpOnly"));
}

#[tokio::test]
async fn post_login_devuelve_401_si_password_incorrecta() {
    let ctx = setup().await;

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": "login2@uniovi.es", "password": "Password123" }))
        .await;

    let response = ctx
        .server
        .post("/auth/login")
        .json(&json!({ "email": "login2@uniovi.es", "password": "wrongpassword" }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn post_login_devuelve_401_si_email_no_existe() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/auth/login")
        .json(&json!({ "email": "noexiste@uniovi.es", "password": "Password123" }))
        .await;

    // Mismo error que contraseña incorrecta — no filtramos qué emails existen
    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn post_login_devuelve_403_si_email_no_verificado() {
    let ctx = setup().await;

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": "noverificado@uniovi.es", "password": "Password123" }))
        .await;

    let response = ctx
        .server
        .post("/auth/login")
        .json(&json!({ "email": "noverificado@uniovi.es", "password": "Password123" }))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"], "email_not_verified");
}

#[tokio::test]
async fn post_login_normaliza_email_a_minusculas() {
    let ctx = setup().await;

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": "case@uniovi.es", "password": "Password123" }))
        .await;
    verify_user(&ctx.pool, "case@uniovi.es").await;

    // Login con el email en mayúsculas debe funcionar
    let response = ctx
        .server
        .post("/auth/login")
        .json(&json!({ "email": "CASE@UNIOVI.ES", "password": "Password123" }))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["user"]["email"], "case@uniovi.es");
}

#[tokio::test]
async fn post_login_devuelve_422_si_password_vacia() {
    let ctx = setup().await;

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": "emptypwd@uniovi.es", "password": "Password123" }))
        .await;

    let response = ctx
        .server
        .post("/auth/login")
        .json(&json!({ "email": "emptypwd@uniovi.es", "password": "" }))
        .await;

    // Contraseña vacía no pasa la validación del handler
    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}
