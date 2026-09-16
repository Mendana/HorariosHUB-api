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

fn base_payload() -> serde_json::Value {
    json!({
        "title": "Entrega del proyecto",
        "subject": "IPS",
        "startTime": "2026-10-01T09:00:00Z",
        "endTime":   "2026-10-01T10:00:00Z"
    })
}

// ─── Autenticación / autorización ─────────────────────────────────────────────

#[tokio::test]
async fn post_events_devuelve_401_sin_autenticar() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;

    let response = ctx.server.post("/events").json(&base_payload()).await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn post_events_devuelve_403_como_student() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "student_ev_create@uniovi.es", "student").await;

    let response = ctx
        .server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&base_payload())
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn post_events_devuelve_201_como_profesor() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_create@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&base_payload())
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    assert_eq!(body["title"], "Entrega del proyecto");
    assert_eq!(body["subject"], "IPS");
    assert_eq!(body["groups"], json!([]));
    assert_eq!(body["createdBy"], "prof_ev_create@uniovi.es");
    assert!(body["recurrence"].is_null());
}

#[tokio::test]
async fn post_events_devuelve_201_como_admin() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "admin_ev_create@uniovi.es", "admin").await;

    let response = ctx
        .server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&base_payload())
        .await;

    response.assert_status(StatusCode::CREATED);
}

// ─── Validación básica ──────────────────────────────────────────────────────────

#[tokio::test]
async fn post_events_devuelve_422_si_falta_title() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_no_title@uniovi.es", "professor").await;

    let mut payload = base_payload();
    payload.as_object_mut().unwrap().remove("title");

    let response = ctx
        .server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&payload)
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_events_devuelve_400_si_end_time_no_es_posterior_a_start_time() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_bad_time@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "title": "Evento",
            "subject": "IPS",
            "startTime": "2026-10-01T10:00:00Z",
            "endTime":   "2026-10-01T09:00:00Z"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_events_devuelve_400_si_la_asignatura_no_existe() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_ev_no_subject@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "title": "Evento",
            "subject": "NO_EXISTE",
            "startTime": "2026-10-01T09:00:00Z",
            "endTime":   "2026-10-01T10:00:00Z"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

// ─── Asociación a asignatura / grupos ──────────────────────────────────────────

#[tokio::test]
async fn post_events_sin_groups_aplica_a_toda_la_asignatura() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1", "T.2"]).await;
    let token = login_as(&ctx, "prof_ev_whole@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&base_payload())
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    assert_eq!(body["groups"], json!([]));
}

#[tokio::test]
async fn post_events_con_groups_aplica_solo_a_esos_grupos() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1", "T.2", "S.1"]).await;
    let token = login_as(&ctx, "prof_ev_specific@uniovi.es", "professor").await;

    let mut payload = base_payload();
    payload["groups"] = json!(["T.1", "T.2"]);

    let response = ctx
        .server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&payload)
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    let mut groups: Vec<String> = body["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    groups.sort();
    assert_eq!(groups, vec!["T.1".to_string(), "T.2".to_string()]);
}

#[tokio::test]
async fn post_events_devuelve_400_si_un_grupo_no_existe_en_la_asignatura() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_bad_group@uniovi.es", "professor").await;

    let mut payload = base_payload();
    payload["groups"] = json!(["T.1", "GRUPO_INVENTADO"]);

    let response = ctx
        .server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&payload)
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

// ─── Recurrencia ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn post_events_con_recurrence_devuelve_recurrencia_en_la_respuesta() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_recurring@uniovi.es", "professor").await;

    let mut payload = base_payload();
    payload["recurrence"] = json!({ "interval": "weekly", "endDate": "2026-12-01" });

    let response = ctx
        .server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&payload)
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    assert_eq!(body["recurrence"]["interval"], "weekly");
    assert_eq!(body["recurrence"]["endDate"], "2026-12-01");
}

#[tokio::test]
async fn post_events_devuelve_400_si_recurrence_end_date_es_anterior_a_start_time() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_bad_recurrence@uniovi.es", "professor").await;

    let mut payload = base_payload();
    payload["recurrence"] = json!({ "interval": "weekly", "endDate": "2026-01-01" });

    let response = ctx
        .server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&payload)
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}
