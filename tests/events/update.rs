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
    payload: serde_json::Value,
) -> serde_json::Value {
    let response = ctx
        .server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&payload)
        .await;
    response.assert_status(StatusCode::CREATED);
    response.json()
}

fn base_payload() -> serde_json::Value {
    json!({
        "title": "Entrega del proyecto",
        "subject": "IPS",
        "startTime": "2026-10-01T09:00:00Z",
        "endTime":   "2026-10-01T10:00:00Z"
    })
}

#[tokio::test]
async fn patch_events_devuelve_401_sin_autenticar() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_upd_setup1@uniovi.es", "professor").await;
    let event = create_event(&ctx, &token, base_payload()).await;
    let id = event["id"].as_str().unwrap();

    let response = ctx
        .server
        .patch(&format!("/events/{id}"))
        .json(&json!({ "title": "Nuevo título" }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn patch_events_devuelve_403_como_student() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let prof_token = login_as(&ctx, "prof_ev_upd_setup2@uniovi.es", "professor").await;
    let event = create_event(&ctx, &prof_token, base_payload()).await;
    let id = event["id"].as_str().unwrap();

    let student_token = login_as(&ctx, "student_ev_upd@uniovi.es", "student").await;

    let response = ctx
        .server
        .patch(&format!("/events/{id}"))
        .add_header("Authorization", format!("Bearer {student_token}"))
        .json(&json!({ "title": "Nuevo título" }))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn patch_events_devuelve_404_si_no_existe() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_ev_upd_404@uniovi.es", "professor").await;

    let response = ctx
        .server
        .patch(&format!("/events/{}", uuid::Uuid::new_v4()))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "title": "Nuevo título" }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn patch_events_actualiza_titulo_y_classroom() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_upd_title@uniovi.es", "professor").await;
    let event = create_event(&ctx, &token, base_payload()).await;
    let id = event["id"].as_str().unwrap();

    let response = ctx
        .server
        .patch(&format!("/events/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "title": "Título actualizado", "classroom": "Aula 3" }))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["title"], "Título actualizado");
    assert_eq!(body["classroom"], "Aula 3");
    assert_eq!(body["subject"], "IPS");
}

#[tokio::test]
async fn patch_events_reemplaza_groups_por_completo() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1", "T.2", "S.1"]).await;
    let token = login_as(&ctx, "prof_ev_upd_groups@uniovi.es", "professor").await;
    let mut payload = base_payload();
    payload["groups"] = json!(["T.1"]);
    let event = create_event(&ctx, &token, payload).await;
    let id = event["id"].as_str().unwrap();

    let response = ctx
        .server
        .patch(&format!("/events/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "groups": ["T.2", "S.1"] }))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    let mut groups: Vec<String> = body["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    groups.sort();
    assert_eq!(groups, vec!["S.1".to_string(), "T.2".to_string()]);
}

#[tokio::test]
async fn patch_events_con_groups_vacio_pasa_a_toda_la_asignatura() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1", "T.2"]).await;
    let token = login_as(&ctx, "prof_ev_upd_clear_groups@uniovi.es", "professor").await;
    let mut payload = base_payload();
    payload["groups"] = json!(["T.1"]);
    let event = create_event(&ctx, &token, payload).await;
    let id = event["id"].as_str().unwrap();

    let response = ctx
        .server
        .patch(&format!("/events/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "groups": [] }))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["groups"], json!([]));
}

#[tokio::test]
async fn patch_events_sin_groups_no_toca_los_existentes() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1", "T.2"]).await;
    let token = login_as(&ctx, "prof_ev_upd_keep_groups@uniovi.es", "professor").await;
    let mut payload = base_payload();
    payload["groups"] = json!(["T.1"]);
    let event = create_event(&ctx, &token, payload).await;
    let id = event["id"].as_str().unwrap();

    let response = ctx
        .server
        .patch(&format!("/events/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "title": "Solo cambio el título" }))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["groups"], json!(["T.1"]));
}

#[tokio::test]
async fn patch_events_devuelve_400_si_la_nueva_asignatura_no_existe() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_upd_bad_subject@uniovi.es", "professor").await;
    let event = create_event(&ctx, &token, base_payload()).await;
    let id = event["id"].as_str().unwrap();

    let response = ctx
        .server
        .patch(&format!("/events/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "subject": "NO_EXISTE" }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn patch_events_puede_anadir_recurrencia_a_evento_puntual() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_upd_add_recurrence@uniovi.es", "professor").await;
    let event = create_event(&ctx, &token, base_payload()).await;
    let id = event["id"].as_str().unwrap();
    assert!(event["recurrence"].is_null());

    let response = ctx
        .server
        .patch(&format!("/events/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "recurrence": { "interval": "monthly", "endDate": "2027-01-01" } }))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["recurrence"]["interval"], "monthly");
}
