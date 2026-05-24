// ── GET /proposals/mine ───────────────────────────────────────────────────────
use crate::common::{login_as, setup};
use axum::http::StatusCode;
use serde_json::json;

// ─── Helper ───────────────────────────────────────────────────────────────────

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

// ─── Autenticación ────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_mine_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx.server.get("/proposals/mine").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_mine_devuelve_200_como_student() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student_mine@uniovi.es", "student").await;

    let response = ctx
        .server
        .get("/proposals/mine")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    // Cualquier usuario autenticado puede ver sus propias propuestas
    response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn get_mine_devuelve_200_como_profesor() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_mine@uniovi.es", "professor").await;

    let response = ctx
        .server
        .get("/proposals/mine")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
}

// ─── Aislamiento: solo ve las suyas ──────────────────────────────────────────

#[tokio::test]
async fn get_mine_devuelve_solo_propuestas_del_usuario() {
    let ctx = setup().await;
    let token_a = login_as(&ctx, "user_a_mine@uniovi.es", "professor").await;
    let token_b = login_as(&ctx, "user_b_mine@uniovi.es", "professor").await;

    // A crea 2 propuestas, B crea 1
    crear_propuesta(&ctx, &token_a).await;
    crear_propuesta(&ctx, &token_a).await;
    crear_propuesta(&ctx, &token_b).await;

    let response_a = ctx
        .server
        .get("/proposals/mine")
        .add_header("Authorization", format!("Bearer {token_a}"))
        .await
        .json::<serde_json::Value>();

    let response_b = ctx
        .server
        .get("/proposals/mine")
        .add_header("Authorization", format!("Bearer {token_b}"))
        .await
        .json::<serde_json::Value>();

    assert_eq!(response_a["total"], 2, "A debe ver solo sus 2 propuestas");
    assert_eq!(response_b["total"], 1, "B debe ver solo su 1 propuesta");
}

#[tokio::test]
async fn get_mine_author_siempre_es_el_usuario_autenticado() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_author_mine@uniovi.es", "professor").await;

    crear_propuesta(&ctx, &token).await;
    crear_propuesta(&ctx, &token).await;

    let body = ctx
        .server
        .get("/proposals/mine")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    for proposal in body["data"].as_array().unwrap() {
        assert_eq!(
            proposal["author"], "prof_author_mine@uniovi.es",
            "el campo author debe ser siempre el del usuario autenticado"
        );
    }
}

#[tokio::test]
async fn get_mine_sin_propuestas_devuelve_lista_vacia() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_empty_mine@uniovi.es", "professor").await;

    // Otro usuario crea propuestas — no deben aparecer
    let other_token = login_as(&ctx, "other_mine@uniovi.es", "professor").await;
    crear_propuesta(&ctx, &other_token).await;

    let body = ctx
        .server
        .get("/proposals/mine")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    assert_eq!(body["data"], json!([]));
    assert_eq!(body["total"], 0);
}

// ─── Estructura de la respuesta ───────────────────────────────────────────────

#[tokio::test]
async fn get_mine_respuesta_tiene_estructura_correcta() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_struct_mine@uniovi.es", "professor").await;

    crear_propuesta(&ctx, &token).await;

    let body = ctx
        .server
        .get("/proposals/mine")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    // Paginación
    assert!(body["total"].is_number(), "falta 'total'");
    assert!(body["page"].is_number(), "falta 'page'");
    assert!(body["limit"].is_number(), "falta 'limit'");
    assert!(body["data"].is_array(), "falta 'data'");

    // Campos de cada propuesta
    let p = &body["data"][0];
    assert!(p["id"].is_string(), "falta 'id'");
    assert!(p["action"].is_string(), "falta 'action'");
    assert!(p["status"].is_string(), "falta 'status'");
    assert!(p["author"].is_string(), "falta 'author'");
    assert!(p["createdAt"].is_string(), "falta 'createdAt'");
    assert!(p["old"].is_object(), "falta 'old'");
    assert!(p["new"].is_object(), "falta 'new'");
}

#[tokio::test]
async fn get_mine_devuelve_propuestas_de_todos_los_estados() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_all_states_mine@uniovi.es", "professor").await;

    let id1 = crear_propuesta(&ctx, &token).await;
    let id2 = crear_propuesta(&ctx, &token).await;
    crear_propuesta(&ctx, &token).await;

    // Cambiamos estado de dos directamente en la BD
    sqlx::query(&format!(
        "UPDATE changes SET change_status = 'approved'::change_status WHERE id = '{id1}'"
    ))
    .execute(&ctx.pool)
    .await
    .unwrap();

    sqlx::query(&format!(
        "UPDATE changes SET change_status = 'rejected'::change_status WHERE id = '{id2}'"
    ))
    .execute(&ctx.pool)
    .await
    .unwrap();

    let body = ctx
        .server
        .get("/proposals/mine")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    // Las 3 propuestas deben aparecer independientemente del estado
    assert_eq!(
        body["total"], 3,
        "mine devuelve todos los estados del usuario"
    );
}

// ─── Paginación ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_mine_limit_restringe_resultados() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_limit_mine@uniovi.es", "professor").await;

    crear_propuesta(&ctx, &token).await;
    crear_propuesta(&ctx, &token).await;
    crear_propuesta(&ctx, &token).await;

    let body = ctx
        .server
        .get("/proposals/mine?limit=2&page=1")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert_eq!(body["total"], 3, "total refleja el total real");
    assert_eq!(body["limit"], 2);
    assert_eq!(body["page"], 1);
}

#[tokio::test]
async fn get_mine_pagina_2_devuelve_el_siguiente_elemento() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_page2_mine@uniovi.es", "professor").await;

    crear_propuesta(&ctx, &token).await;
    crear_propuesta(&ctx, &token).await;
    crear_propuesta(&ctx, &token).await;

    let page1 = ctx
        .server
        .get("/proposals/mine?limit=2&page=1")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    let page2 = ctx
        .server
        .get("/proposals/mine?limit=2&page=2")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    assert_eq!(page2["data"].as_array().unwrap().len(), 1);

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
async fn get_mine_defaults_pagina_1_y_limit_10() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_defaults_mine@uniovi.es", "professor").await;

    let body = ctx
        .server
        .get("/proposals/mine")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    assert_eq!(body["page"], 1);
    assert_eq!(body["limit"], 10);
}
