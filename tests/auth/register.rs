use crate::common::setup;
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn post_register_devuelve_201() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": "diego@uniovi.es",
            "password": "password123"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);

    let body: serde_json::Value = response.json();
    assert_eq!(body["email"], "diego@uniovi.es");
    assert_eq!(body["role"], "student");
    assert!(body["id"].is_string());
}

#[tokio::test]
async fn post_register_devuelve_409_si_email_duplicado() {
    let ctx = setup().await;

    let payload = json!({
        "email": "diego@uniovi.es",
        "password": "password123"
    });

    ctx.server.post("/auth/register").json(&payload).await;

    let response = ctx.server.post("/auth/register").json(&payload).await;
    response.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn post_register_devuelve_400_si_email_invalido() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": "esto-no-es-un-email",
            "password": "password123"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_register_devuelve_400_si_password_corta() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": "diego@uniovi.es",
            "password": "corta"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}
