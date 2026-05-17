use crate::common::setup;
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn post_login_devuelve_200_y_cookie() {
    let ctx = setup().await;

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": "login@uniovi.es", "password": "password123" }))
        .await;

    let response = ctx
        .server
        .post("/auth/login")
        .json(&json!({ "email": "login@uniovi.es", "password": "password123" }))
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
        .json(&json!({ "email": "login2@uniovi.es", "password": "password123" }))
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
        .json(&json!({ "email": "noexiste@uniovi.es", "password": "password123" }))
        .await;

    // Mismo error que contraseña incorrecta — no filtramos qué emails existen
    response.assert_status(StatusCode::UNAUTHORIZED);
}
