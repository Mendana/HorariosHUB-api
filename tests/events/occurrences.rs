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

#[tokio::test]
async fn get_occurrences_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx
        .server
        .get("/events/occurrences?from=2026-10-01T00:00:00Z&to=2026-10-08T00:00:00Z")
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_occurrences_devuelve_400_si_to_no_es_posterior_a_from() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_ev_occ_bad_range@uniovi.es", "professor").await;

    let response = ctx
        .server
        .get("/events/occurrences?from=2026-10-08T00:00:00Z&to=2026-10-01T00:00:00Z")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_occurrences_devuelve_400_si_el_rango_supera_366_dias() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_ev_occ_too_long@uniovi.es", "professor").await;

    let response = ctx
        .server
        .get("/events/occurrences?from=2026-01-01T00:00:00Z&to=2028-01-01T00:00:00Z")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_occurrences_incluye_evento_puntual_dentro_del_rango() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_occ_single@uniovi.es", "professor").await;

    ctx.server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "title": "Evento puntual",
            "subject": "IPS",
            "groups": ["T.1"],
            "startTime": "2026-10-05T09:00:00Z",
            "endTime":   "2026-10-05T10:00:00Z"
        }))
        .await
        .assert_status(StatusCode::CREATED);

    let response = ctx
        .server
        .get("/events/occurrences?from=2026-10-01T00:00:00Z&to=2026-10-08T00:00:00Z")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    let occs = body["occurrences"].as_array().unwrap();
    assert_eq!(occs.len(), 1);
    assert_eq!(occs[0]["title"], "Evento puntual");
    assert_eq!(occs[0]["isRecurring"], false);
    assert_eq!(occs[0]["groups"], json!(["T.1"]));
}

#[tokio::test]
async fn get_occurrences_no_incluye_evento_puntual_fuera_de_rango() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_occ_out_of_range@uniovi.es", "professor").await;

    ctx.server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "title": "Fuera de rango",
            "subject": "IPS",
            "startTime": "2026-12-05T09:00:00Z",
            "endTime":   "2026-12-05T10:00:00Z"
        }))
        .await
        .assert_status(StatusCode::CREATED);

    let response = ctx
        .server
        .get("/events/occurrences?from=2026-10-01T00:00:00Z&to=2026-10-08T00:00:00Z")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["occurrences"], json!([]));
}

#[tokio::test]
async fn get_occurrences_expande_evento_semanal_recurrente() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_occ_weekly@uniovi.es", "professor").await;

    ctx.server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "title": "Tutoría semanal",
            "subject": "IPS",
            "startTime": "2026-10-01T09:00:00Z",
            "endTime":   "2026-10-01T10:00:00Z",
            "recurrence": { "interval": "weekly", "endDate": "2026-12-01" }
        }))
        .await
        .assert_status(StatusCode::CREATED);

    // Rango de 4 semanas: debería haber 4 ocurrencias (01, 08, 15, 22 oct)
    let response = ctx
        .server
        .get("/events/occurrences?from=2026-10-01T00:00:00Z&to=2026-10-29T00:00:00Z")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    let occs = body["occurrences"].as_array().unwrap();
    assert_eq!(occs.len(), 4);
    assert!(occs.iter().all(|o| o["isRecurring"] == true));

    let mut starts: Vec<String> = occs
        .iter()
        .map(|o| o["startTime"].as_str().unwrap().to_string())
        .collect();
    starts.sort();
    assert_eq!(
        starts,
        vec![
            "2026-10-01T09:00:00Z",
            "2026-10-08T09:00:00Z",
            "2026-10-15T09:00:00Z",
            "2026-10-22T09:00:00Z",
        ]
    );
}

#[tokio::test]
async fn get_occurrences_respeta_recurrence_end_date() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_occ_end_date@uniovi.es", "professor").await;

    ctx.server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "title": "Recurrente con fin",
            "subject": "IPS",
            "startTime": "2026-10-01T09:00:00Z",
            "endTime":   "2026-10-01T10:00:00Z",
            "recurrence": { "interval": "daily", "endDate": "2026-10-03" }
        }))
        .await
        .assert_status(StatusCode::CREATED);

    let response = ctx
        .server
        .get("/events/occurrences?from=2026-10-01T00:00:00Z&to=2026-10-10T00:00:00Z")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    let occs = body["occurrences"].as_array().unwrap();
    // 01, 02, 03 oct -> 3 ocurrencias, ninguna después del 3
    assert_eq!(occs.len(), 3);
}

#[tokio::test]
async fn get_occurrences_filtra_por_subject() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    seed_subject(&ctx, "ALG", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_occ_subject_filter@uniovi.es", "professor").await;

    for subject in ["IPS", "ALG"] {
        ctx.server
            .post("/events")
            .add_header("Authorization", format!("Bearer {token}"))
            .json(&json!({
                "title": format!("Evento {subject}"),
                "subject": subject,
                "startTime": "2026-10-05T09:00:00Z",
                "endTime":   "2026-10-05T10:00:00Z"
            }))
            .await
            .assert_status(StatusCode::CREATED);
    }

    let response = ctx
        .server
        .get("/events/occurrences?from=2026-10-01T00:00:00Z&to=2026-10-08T00:00:00Z&subject=IPS")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    let occs = body["occurrences"].as_array().unwrap();
    assert_eq!(occs.len(), 1);
    assert_eq!(occs[0]["subject"], "IPS");
}
