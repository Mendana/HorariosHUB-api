use crate::common::{login_as, setup};
use axum::http::StatusCode;
use serde_json::json;

/// Crea una propuesta de tipo "create" via API y devuelve su id como String.
async fn crear_propuesta(ctx: &crate::common::TestContext, token: &str) -> String {
    let response = ctx
        .server
        .post("/proposals")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "changeType": "create",
            "changes": {
                "subject": "ALG",
                "grp": "Teoría",
                "newStartsAt": "2025-09-15T09:00:00Z",
                "newDuration": 90,
                "newClassroom": "Aula 101"
            }
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
    response.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Cambia el estado de una propuesta directamente en la base de datos.
async fn cambiar_estado(ctx: &crate::common::TestContext, id: &str, status: &str) {
    sqlx::query(&format!(
        "UPDATE changes SET change_status = '{status}'::change_status WHERE id = '{id}'"
    ))
    .execute(&ctx.pool)
    .await
    .unwrap();
}

/// Mueve una propuesta de `changes` a `changes_history`, simulando lo que hace
/// el sync del scraper al archivar un cambio (aprobado huérfano o rechazado).
async fn archivar_propuesta(ctx: &crate::common::TestContext, id: &str) {
    sqlx::query(&format!(
        r#"
        INSERT INTO changes_history (
            id, proposed_by, session_id, subject, grp, change_type, change_status,
            prev_starts_at, prev_duration, prev_classroom,
            new_starts_at, new_duration, new_classroom, proposed_at
        )
        SELECT
            id, proposed_by, session_id, subject, grp, change_type, change_status,
            prev_starts_at, prev_duration, prev_classroom,
            new_starts_at, new_duration, new_classroom, proposed_at
        FROM changes WHERE id = '{id}'
        "#
    ))
    .execute(&ctx.pool)
    .await
    .unwrap();

    sqlx::query(&format!("DELETE FROM changes WHERE id = '{id}'"))
        .execute(&ctx.pool)
        .await
        .unwrap();
}

// ─── Autenticación / autorización ─────────────────────────────────────────────

#[tokio::test]
async fn get_history_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx.server.get("/proposals/history").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_history_devuelve_403_como_student() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student_history@uniovi.es", "student").await;

    let response = ctx
        .server
        .get("/proposals/history")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn get_history_devuelve_200_como_profesor() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_history@uniovi.es", "professor").await;

    let response = ctx
        .server
        .get("/proposals/history")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
}

// ─── Contenido: pending no aparece, approved/rejected sí ──────────────────────

#[tokio::test]
async fn get_history_no_incluye_propuestas_pendientes() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_history_pending@uniovi.es", "professor").await;

    crear_propuesta(&ctx, &token).await; // queda pending

    let response = ctx
        .server
        .get("/proposals/history")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 0);
}

#[tokio::test]
async fn get_history_incluye_propuestas_aprobadas_y_rechazadas_vigentes() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_history_live@uniovi.es", "professor").await;

    let id1 = crear_propuesta(&ctx, &token).await;
    let id2 = crear_propuesta(&ctx, &token).await;
    cambiar_estado(&ctx, &id1, "approved").await;
    cambiar_estado(&ctx, &id2, "rejected").await;

    let response = ctx
        .server
        .get("/proposals/history")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 2);
    for entry in body["data"].as_array().unwrap() {
        assert!(entry["archivedAt"].is_null());
    }
}

#[tokio::test]
async fn get_history_incluye_propuestas_archivadas_con_archived_at() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_history_archived@uniovi.es", "professor").await;

    let id = crear_propuesta(&ctx, &token).await;
    cambiar_estado(&ctx, &id, "approved").await;
    archivar_propuesta(&ctx, &id).await;

    let response = ctx
        .server
        .get("/proposals/history")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 1);
    assert_eq!(body["data"][0]["id"], id);
    assert_eq!(body["data"][0]["status"], "approved");
    assert!(body["data"][0]["archivedAt"].is_string());
}

// ─── Filtrado por estado ──────────────────────────────────────────────────────

#[tokio::test]
async fn get_history_filtra_por_status_approved() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_history_filter_a@uniovi.es", "professor").await;

    let id1 = crear_propuesta(&ctx, &token).await;
    let id2 = crear_propuesta(&ctx, &token).await;
    cambiar_estado(&ctx, &id1, "approved").await;
    cambiar_estado(&ctx, &id2, "rejected").await;

    let response = ctx
        .server
        .get("/proposals/history?status=approved")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 1);
    assert_eq!(body["data"][0]["status"], "approved");
}

#[tokio::test]
async fn get_history_filtra_por_status_rejected_incluyendo_archivados() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_history_filter_r@uniovi.es", "professor").await;

    let id1 = crear_propuesta(&ctx, &token).await;
    cambiar_estado(&ctx, &id1, "rejected").await;
    archivar_propuesta(&ctx, &id1).await;

    let id2 = crear_propuesta(&ctx, &token).await;
    cambiar_estado(&ctx, &id2, "approved").await;

    let response = ctx
        .server
        .get("/proposals/history?status=rejected")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 1);
    assert_eq!(body["data"][0]["status"], "rejected");
    assert!(body["data"][0]["archivedAt"].is_string());
}

// ─── Paginación ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_history_pagina_los_resultados() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_history_page@uniovi.es", "professor").await;

    for _ in 0..3 {
        let id = crear_propuesta(&ctx, &token).await;
        cambiar_estado(&ctx, &id, "approved").await;
    }

    let response = ctx
        .server
        .get("/proposals/history?limit=2&page=1")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert_eq!(body["total"], 3);
    assert_eq!(body["limit"], 2);
    assert_eq!(body["page"], 1);
}
