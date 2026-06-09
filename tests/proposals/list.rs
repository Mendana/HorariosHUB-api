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

// ─── Autenticación / autorización ─────────────────────────────────────────────

#[tokio::test]
async fn get_proposals_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx.server.get("/proposals").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_proposals_devuelve_403_como_student() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student_list@uniovi.es", "student").await;

    let response = ctx
        .server
        .get("/proposals")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn get_proposals_devuelve_200_como_profesor() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_list@uniovi.es", "professor").await;

    let response = ctx
        .server
        .get("/proposals")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn get_proposals_devuelve_200_como_admin() {
    let ctx = setup().await;
    let token = login_as(&ctx, "admin_list@uniovi.es", "admin").await;

    let response = ctx
        .server
        .get("/proposals")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
}

// ─── Estructura de la respuesta ───────────────────────────────────────────────

#[tokio::test]
async fn get_proposals_sin_datos_devuelve_lista_vacia() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_empty@uniovi.es", "professor").await;

    let response = ctx
        .server
        .get("/proposals")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"], json!([]));
    assert_eq!(body["total"], 0);
    assert_eq!(body["page"], 1);
    assert_eq!(body["limit"], 10);
}

#[tokio::test]
async fn get_proposals_respuesta_tiene_estructura_correcta() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_struct@uniovi.es", "professor").await;

    crear_propuesta(&ctx, &token).await;

    let response = ctx
        .server
        .get("/proposals")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    // Campos de paginación
    assert!(body["total"].is_number(), "falta campo 'total'");
    assert!(body["page"].is_number(), "falta campo 'page'");
    assert!(body["limit"].is_number(), "falta campo 'limit'");
    assert!(body["data"].is_array(), "falta campo 'data'");

    // Campos de cada propuesta
    let proposal = &body["data"][0];
    assert!(proposal["id"].is_string(), "falta campo 'id'");
    assert!(proposal["action"].is_string(), "falta campo 'action'");
    assert!(proposal["status"].is_string(), "falta campo 'status'");
    assert!(proposal["author"].is_string(), "falta campo 'author'");
    assert!(proposal["createdAt"].is_string(), "falta campo 'createdAt'");
    assert!(proposal["old"].is_object(), "falta campo 'old'");
    assert!(proposal["new"].is_object(), "falta campo 'new'");
}

#[tokio::test]
async fn get_proposals_author_es_el_email_del_creador() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_author@uniovi.es", "professor").await;

    crear_propuesta(&ctx, &token).await;

    let response = ctx
        .server
        .get("/proposals")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"][0]["author"], "prof_author@uniovi.es");
}

#[tokio::test]
async fn get_proposals_create_tiene_snapshot_correcto() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_snap@uniovi.es", "professor").await;

    crear_propuesta(&ctx, &token).await;

    let response = ctx
        .server
        .get("/proposals")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    let body: serde_json::Value = response.json();
    let proposal = &body["data"][0];

    assert_eq!(proposal["action"], "create");
    // Para una propuesta de creación, old está vacío
    assert!(
        proposal["old"]["startsAt"].is_null(),
        "old.startsAt debería ser null en un create"
    );
    // new tiene los valores enviados
    assert_eq!(proposal["new"]["classroom"], "Aula 101");
    assert_eq!(proposal["new"]["subject"], "ALG");
    assert_eq!(proposal["new"]["grp"], "Teoría");
}

// ─── Filtrado por estado ──────────────────────────────────────────────────────

#[tokio::test]
async fn get_proposals_filtra_por_status_pending() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_filter_p@uniovi.es", "professor").await;

    let id = crear_propuesta(&ctx, &token).await;
    cambiar_estado(&ctx, &id, "approved").await;
    crear_propuesta(&ctx, &token).await; // esta queda pending

    let response = ctx
        .server
        .get("/proposals?status=pending")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 1);
    assert_eq!(body["data"][0]["status"], "pending");
}

#[tokio::test]
async fn get_proposals_filtra_por_status_approved() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_filter_a@uniovi.es", "professor").await;

    let id1 = crear_propuesta(&ctx, &token).await;
    let id2 = crear_propuesta(&ctx, &token).await;
    cambiar_estado(&ctx, &id1, "approved").await;
    cambiar_estado(&ctx, &id2, "approved").await;
    crear_propuesta(&ctx, &token).await; // esta queda pending, no debe aparecer

    let response = ctx
        .server
        .get("/proposals?status=approved")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 2);
    for p in body["data"].as_array().unwrap() {
        assert_eq!(p["status"], "approved");
    }
}

#[tokio::test]
async fn get_proposals_filtra_por_status_rejected() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_filter_r@uniovi.es", "professor").await;

    let id = crear_propuesta(&ctx, &token).await;
    cambiar_estado(&ctx, &id, "rejected").await;
    crear_propuesta(&ctx, &token).await; // queda pending

    let response = ctx
        .server
        .get("/proposals?status=rejected")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 1);
    assert_eq!(body["data"][0]["status"], "rejected");
}

#[tokio::test]
async fn get_proposals_status_all_devuelve_todas() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_filter_all@uniovi.es", "professor").await;

    let id1 = crear_propuesta(&ctx, &token).await;
    let id2 = crear_propuesta(&ctx, &token).await;
    let _id3 = crear_propuesta(&ctx, &token).await;
    cambiar_estado(&ctx, &id1, "approved").await;
    cambiar_estado(&ctx, &id2, "rejected").await;
    // id3 queda pending

    let response = ctx
        .server
        .get("/proposals?status=all")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 3);
}

#[tokio::test]
async fn get_proposals_sin_status_devuelve_todas() {
    // Sin ?status=, el comportamiento por defecto es devolver todas las propuestas
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_no_status@uniovi.es", "professor").await;

    let id = crear_propuesta(&ctx, &token).await;
    cambiar_estado(&ctx, &id, "approved").await;
    crear_propuesta(&ctx, &token).await; // pending

    let response = ctx
        .server
        .get("/proposals")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 2, "sin filtro deben aparecer todas");
}

// ─── Paginación ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_proposals_limit_restringe_resultados() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_page1@uniovi.es", "professor").await;

    crear_propuesta(&ctx, &token).await;
    crear_propuesta(&ctx, &token).await;
    crear_propuesta(&ctx, &token).await;

    let response = ctx
        .server
        .get("/proposals?limit=2&page=1")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert_eq!(
        body["total"], 3,
        "total refleja el total real, no la página"
    );
    assert_eq!(body["limit"], 2);
    assert_eq!(body["page"], 1);
}

#[tokio::test]
async fn get_proposals_pagina_2_devuelve_el_siguiente_elemento() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_page2@uniovi.es", "professor").await;

    // Creamos 3 propuestas; la paginación por defecto ordena por fecha DESC
    crear_propuesta(&ctx, &token).await;
    crear_propuesta(&ctx, &token).await;
    crear_propuesta(&ctx, &token).await;

    let page1 = ctx
        .server
        .get("/proposals?limit=2&page=1")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    let page2 = ctx
        .server
        .get("/proposals?limit=2&page=2")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    // La página 2 tiene el elemento restante
    assert_eq!(page2["data"].as_array().unwrap().len(), 1);

    // Los ids de página 1 y página 2 no se solapan
    let ids_p1: Vec<&str> = page1["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    let id_p2 = page2["data"][0]["id"].as_str().unwrap();
    assert!(!ids_p1.contains(&id_p2), "las páginas no deben solaparse");
}

#[tokio::test]
async fn get_proposals_pagina_fuera_de_rango_devuelve_lista_vacia() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_page_oor@uniovi.es", "professor").await;

    crear_propuesta(&ctx, &token).await;

    let response = ctx
        .server
        .get("/proposals?limit=10&page=99")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["total"], 1, "total sigue reflejando el total real");
}
