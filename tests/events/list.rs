use crate::common::{login_as, setup};
use axum::http::StatusCode;
use serde_json::json;

async fn seed_subject(ctx: &crate::common::TestContext, subject: &str, groups: &[&str]) {
    for grp in groups {
        sqlx::query!(
            "INSERT INTO subject_groups (subject, grp) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            subject,
            grp
        )
        .execute(&ctx.pool)
        .await
        .unwrap();
    }
}

async fn create_event(
    ctx: &crate::common::TestContext,
    token: &str,
    title: &str,
    subject: &str,
) {
    ctx.server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "title": title,
            "subject": subject,
            "startTime": "2026-10-01T09:00:00Z",
            "endTime":   "2026-10-01T10:00:00Z"
        }))
        .await
        .assert_status(StatusCode::CREATED);
}

#[tokio::test]
async fn get_events_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx.server.get("/events").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_events_sin_datos_devuelve_lista_vacia() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_ev_list_empty@uniovi.es", "professor").await;

    let response = ctx
        .server
        .get("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["events"], json!([]));
    assert_eq!(body["total"], 0);
}

#[tokio::test]
async fn get_events_devuelve_200_como_student() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student_ev_list@uniovi.es", "student").await;

    let response = ctx
        .server
        .get("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn get_events_filtra_por_subject() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    seed_subject(&ctx, "ALG", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_list_filter@uniovi.es", "professor").await;

    create_event(&ctx, &token, "Evento IPS", "IPS").await;
    create_event(&ctx, &token, "Evento ALG", "ALG").await;

    let response = ctx
        .server
        .get("/events?subject=IPS")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 1);
    assert_eq!(body["events"][0]["subject"], "IPS");
}

#[tokio::test]
async fn get_events_filtra_por_search_en_title() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_list_search@uniovi.es", "professor").await;

    create_event(&ctx, &token, "Examen parcial", "IPS").await;
    create_event(&ctx, &token, "Entrega de proyecto", "IPS").await;

    let response = ctx
        .server
        .get("/events?search=parcial")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 1);
    assert_eq!(body["events"][0]["title"], "Examen parcial");
}

#[tokio::test]
async fn get_events_pagina_los_resultados() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_list_page@uniovi.es", "professor").await;

    for i in 0..3 {
        create_event(&ctx, &token, &format!("Evento {i}"), "IPS").await;
    }

    let response = ctx
        .server
        .get("/events?limit=2&page=1")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["events"].as_array().unwrap().len(), 2);
    assert_eq!(body["total"], 3);
}

#[tokio::test]
async fn get_events_by_id_devuelve_200_y_detalle() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_get_one@uniovi.es", "professor").await;

    let created: serde_json::Value = ctx
        .server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "title": "Detalle",
            "subject": "IPS",
            "groups": ["T.1"],
            "startTime": "2026-10-01T09:00:00Z",
            "endTime":   "2026-10-01T10:00:00Z"
        }))
        .await
        .json();
    let id = created["id"].as_str().unwrap();

    let response = ctx
        .server
        .get(&format!("/events/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["title"], "Detalle");
    assert_eq!(body["groups"], json!(["T.1"]));
}

#[tokio::test]
async fn get_events_by_id_devuelve_404_si_no_existe() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_ev_get_404@uniovi.es", "professor").await;

    let response = ctx
        .server
        .get(&format!("/events/{}", uuid::Uuid::new_v4()))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}
