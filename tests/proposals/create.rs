use crate::common::{create_test_session, login_as, setup};
use axum::http::StatusCode;
use serde_json::json;

// ── CREATE ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn post_proposals_create_devuelve_201_como_profesor() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof@uniovi.es", "professor").await;

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

    let body: serde_json::Value = response.json();
    assert!(body["id"].is_string());
    assert_eq!(body["status"], "pending");
    assert!(body["createdAt"].is_string());
}

#[tokio::test]
async fn post_proposals_create_devuelve_201_como_student() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student@uniovi.es", "student").await;

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
}

#[tokio::test]
async fn post_proposals_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/proposals")
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

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn post_proposals_create_devuelve_422_si_duracion_no_es_multiplo_de_30() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof2@uniovi.es", "professor").await;

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
                "newDuration": 45,
                "newClassroom": "Aula 101"
            }
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_proposals_create_devuelve_422_si_duracion_menor_de_30() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof3@uniovi.es", "professor").await;

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
                "newDuration": 15,
                "newClassroom": "Aula 101"
            }
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_proposals_create_devuelve_422_si_subject_vacio() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof4@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/proposals")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "changeType": "create",
            "changes": {
                "subject": "",
                "grp": "Teoría",
                "newStartsAt": "2025-09-15T09:00:00Z",
                "newDuration": 90,
                "newClassroom": "Aula 101"
            }
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_proposals_create_devuelve_422_si_grp_vacio() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof5@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/proposals")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "changeType": "create",
            "changes": {
                "subject": "ALG",
                "grp": "",
                "newStartsAt": "2025-09-15T09:00:00Z",
                "newDuration": 90,
                "newClassroom": "Aula 101"
            }
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

// ── MODIFY ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn post_proposals_modify_devuelve_201() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof6@uniovi.es", "professor").await;
    let session_id = create_test_session(&ctx.pool).await;

    let response = ctx
        .server
        .post("/proposals")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "changeType": "modify",
            "changes": {
                "sessionId": session_id,
                "newDuration": 60
            }
        }))
        .await;

    response.assert_status(StatusCode::CREATED);

    let body: serde_json::Value = response.json();
    assert!(body["id"].is_string());
    assert_eq!(body["status"], "pending");
}

#[tokio::test]
async fn post_proposals_modify_con_solo_nueva_hora_es_valido() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof8@uniovi.es", "professor").await;
    let session_id = create_test_session(&ctx.pool).await;

    let response = ctx
        .server
        .post("/proposals")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "changeType": "modify",
            "changes": {
                "sessionId": session_id,
                "newStartsAt": "2025-09-15T10:00:00Z"
            }
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
}

#[tokio::test]
async fn post_proposals_modify_sin_ningun_cambio_devuelve_422() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof8b@uniovi.es", "professor").await;
    let session_id = create_test_session(&ctx.pool).await;

    let response = ctx
        .server
        .post("/proposals")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "changeType": "modify",
            "changes": {
                "sessionId": session_id
            }
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_proposals_modify_devuelve_422_si_duracion_no_es_multiplo_de_30() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof7@uniovi.es", "professor").await;
    let session_id = create_test_session(&ctx.pool).await;

    let response = ctx
        .server
        .post("/proposals")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "changeType": "modify",
            "changes": {
                "sessionId": session_id,
                "newDuration": 45
            }
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

// ── DELETE ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn post_proposals_delete_devuelve_201() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof9@uniovi.es", "professor").await;
    let session_id = create_test_session(&ctx.pool).await;

    let response = ctx
        .server
        .post("/proposals")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "changeType": "delete",
            "changes": {
                "sessionId": session_id
            }
        }))
        .await;

    response.assert_status(StatusCode::CREATED);

    let body: serde_json::Value = response.json();
    assert!(body["id"].is_string());
    assert_eq!(body["status"], "pending");
}

#[tokio::test]
async fn post_proposals_delete_session_inexistente_devuelve_404() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof9b@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/proposals")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "changeType": "delete",
            "changes": {
                "sessionId": "00000000-0000-0000-0000-000000000000"
            }
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

// ── PERSISTENCIA ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn post_proposals_persiste_en_base_de_datos() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof10@uniovi.es", "professor").await;

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

    let body: serde_json::Value = response.json();
    let proposal_id: uuid::Uuid = body["id"].as_str().unwrap().parse().unwrap();

    let row = sqlx::query!(
        "SELECT change_type::text, change_status::text, subject, grp FROM changes WHERE id = $1",
        proposal_id
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(row.change_type, Some("create".to_string()));
    assert_eq!(row.change_status, Some("pending".to_string()));
    assert_eq!(row.subject, Some("ALG".to_string()));
    assert_eq!(row.grp, Some("Teoría".to_string()));
}
