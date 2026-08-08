use crate::common::{login_user, setup, verify_user};
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
            "password": "Password123"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);

    let body: serde_json::Value = response.json();
    assert_eq!(body["email"], "diego@uniovi.es");
    assert_eq!(body["role"], "student");
}

#[tokio::test]
async fn post_register_devuelve_409_si_email_duplicado() {
    let ctx = setup().await;

    let payload = json!({
        "email": "diego@uniovi.es",
        "password": "Password123"
    });

    ctx.server.post("/auth/register").json(&payload).await;

    let response = ctx.server.post("/auth/register").json(&payload).await;
    response.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn post_register_devuelve_422_si_email_invalido() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": "esto-no-es-un-email",
            "password": "Password123"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_register_devuelve_422_si_password_corta() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": "diego@uniovi.es",
            "password": "corta"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_register_devuelve_422_si_password_no_tiene_letras() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": "diego@uniovi.es",
            "password": "12345678"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_register_devuelve_422_si_password_no_tiene_numeros() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": "diego@uniovi.es",
            "password": "Password"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_register_devuelve_422_si_password_no_tiene_mayusculas() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": "diego@uniovi.es",
            "password": "password123"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_register_devuelve_422_si_password_no_tiene_minusculas() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": "diego@uniovi.es",
            "password": "PASSWORD123"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_register_normaliza_email_a_minusculas() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({ "email": "UPPER@UNIOVI.ES", "password": "Password123" }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    assert_eq!(body["email"], "upper@uniovi.es");
}

#[tokio::test]
async fn post_register_devuelve_409_email_duplicado_case_insensitive() {
    let ctx = setup().await;

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": "dup@uniovi.es", "password": "Password123" }))
        .await;

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({ "email": "DUP@UNIOVI.ES", "password": "Password123" }))
        .await;

    response.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn post_register_permite_login_tras_registro() {
    let ctx = setup().await;

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": "newuser@uniovi.es", "password": "Password123" }))
        .await
        .assert_status(StatusCode::CREATED);

    verify_user(&ctx.pool, "newuser@uniovi.es").await;

    let token = login_user(&ctx.server, "newuser@uniovi.es", "Password123").await;
    assert!(!token.is_empty());
}
