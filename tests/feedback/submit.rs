use crate::common::setup;
use axum::http::StatusCode;
use serde_json::json;

// ─── Caso feliz ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn post_feedback_devuelve_200_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/feedback")
        .json(&json!({
            "name": "Ana",
            "email": "ana@uniovi.es",
            "subject": "Sugerencia",
            "body": "El horario de mates no se ve bien en móvil."
        }))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert!(body["message"].is_string());
}

#[tokio::test]
async fn post_feedback_sin_subject_devuelve_200() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/feedback")
        .json(&json!({
            "name": "Ana",
            "email": "ana@uniovi.es",
            "body": "Mensaje sin asunto."
        }))
        .await;

    response.assert_status(StatusCode::OK);
}

// ─── Validación ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn post_feedback_devuelve_422_si_falta_name() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/feedback")
        .json(&json!({
            "email": "ana@uniovi.es",
            "body": "Cuerpo del mensaje"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_feedback_devuelve_422_si_name_esta_vacio() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/feedback")
        .json(&json!({
            "name": "",
            "email": "ana@uniovi.es",
            "body": "Cuerpo del mensaje"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_feedback_devuelve_422_si_email_invalido() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/feedback")
        .json(&json!({
            "name": "Ana",
            "email": "no-es-un-email",
            "body": "Cuerpo del mensaje"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_feedback_devuelve_422_si_body_esta_vacio() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/feedback")
        .json(&json!({
            "name": "Ana",
            "email": "ana@uniovi.es",
            "body": ""
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_feedback_devuelve_422_si_falta_body() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/feedback")
        .json(&json!({
            "name": "Ana",
            "email": "ana@uniovi.es"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}
