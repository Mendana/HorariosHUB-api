use crate::common::setup;
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn post_recover_devuelve_200_si_email_existe() {
    let ctx = setup().await;

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": "recover_ok@uniovi.es", "password": "password123" }))
        .await;

    let response = ctx
        .server
        .post("/auth/recover")
        .json(&json!({ "email": "recover_ok@uniovi.es" }))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(
        body["message"],
        "Si el email existe, se ha enviado un enlace de recuperación"
    );

    let token_count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) FROM password_reset_tokens prt
        JOIN users u ON u.id = prt.user_id
        WHERE u.email = $1
        "#,
        "recover_ok@uniovi.es"
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(token_count, Some(1));
}

#[tokio::test]
async fn post_recover_devuelve_200_si_email_no_existe() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/auth/recover")
        .json(&json!({ "email": "noexiste@uniovi.es" }))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(
        body["message"],
        "Si el email existe, se ha enviado un enlace de recuperación"
    );
}

#[tokio::test]
async fn post_recover_no_crea_token_si_email_no_existe() {
    let ctx = setup().await;

    ctx.server
        .post("/auth/recover")
        .json(&json!({ "email": "fantasma@uniovi.es" }))
        .await;

    let count = sqlx::query_scalar!("SELECT COUNT(*) FROM password_reset_tokens")
        .fetch_one(&ctx.pool)
        .await
        .unwrap();

    assert_eq!(count, Some(0));
}

#[tokio::test]
async fn post_recover_invalida_token_anterior_si_se_pide_de_nuevo() {
    let ctx = setup().await;

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": "recover_twice@uniovi.es", "password": "password123" }))
        .await;

    ctx.server
        .post("/auth/recover")
        .json(&json!({ "email": "recover_twice@uniovi.es" }))
        .await;

    ctx.server
        .post("/auth/recover")
        .json(&json!({ "email": "recover_twice@uniovi.es" }))
        .await;

    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) FROM password_reset_tokens prt
        JOIN users u ON u.id = prt.user_id
        WHERE u.email = $1
        "#,
        "recover_twice@uniovi.es"
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(count, Some(1));
}

#[tokio::test]
async fn post_recover_devuelve_422_si_email_invalido() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/auth/recover")
        .json(&json!({ "email": "esto-no-es-un-email" }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}
