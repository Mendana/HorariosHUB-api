use crate::common::setup;
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn get_verify_devuelve_200_con_token_valido() {
    let ctx = setup().await;

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": "verify_ok@uniovi.es", "password": "password123" }))
        .await;

    let token = sqlx::query_scalar!(
        "SELECT token FROM verification_tokens ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    let response = ctx
        .server
        .get(&format!("/auth/verify?token={}", token))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["message"], "Email verificado correctamente");

    let verified = sqlx::query_scalar!(
        "SELECT verified FROM users WHERE email = $1",
        "verify_ok@uniovi.es"
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert!(verified);

    let token_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM verification_tokens WHERE token = $1",
        token
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(token_count, Some(0));
}

#[tokio::test]
async fn get_verify_devuelve_400_con_token_invalido() {
    let ctx = setup().await;

    let response = ctx
        .server
        .get("/auth/verify?token=token-que-no-existe")
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"], "bad_request");
}

#[tokio::test]
async fn get_verify_devuelve_400_sin_parametro_token() {
    let ctx = setup().await;

    // Sin ?token=... el extractor Query falla con 400
    let response = ctx.server.get("/auth/verify").await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_verify_devuelve_400_si_token_ya_usado() {
    let ctx = setup().await;

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": "verify_reuse@uniovi.es", "password": "password123" }))
        .await;

    let token = sqlx::query_scalar!(
        "SELECT token FROM verification_tokens ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    ctx.server
        .get(&format!("/auth/verify?token={}", token))
        .await;

    let response = ctx
        .server
        .get(&format!("/auth/verify?token={}", token))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_verify_devuelve_400_si_token_expirado() {
    let ctx = setup().await;

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": "verify_exp@uniovi.es", "password": "password123" }))
        .await;

    sqlx::query!("UPDATE verification_tokens SET expires_at = NOW() - INTERVAL '1 hour'")
        .execute(&ctx.pool)
        .await
        .unwrap();

    let token = sqlx::query_scalar!(
        "SELECT token FROM verification_tokens ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    let response = ctx
        .server
        .get(&format!("/auth/verify?token={}", token))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"], "bad_request");
}
