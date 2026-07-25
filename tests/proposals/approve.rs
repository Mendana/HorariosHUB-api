// ── APPROVE PROPOSAL ──────────────────────────────────────────────────────────
use crate::common::{create_test_session, login_as, setup};
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn approve_proposal_create_devuelve_200_y_aprueba() {
    let ctx = setup().await;

    let professor_token = login_as(&ctx, "prof_approve@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/proposals")
        .add_header("Authorization", format!("Bearer {professor_token}"))
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
    let proposal_id = body["id"].as_str().unwrap();

    let approve_response = ctx
        .server
        .patch(&format!("/proposals/{proposal_id}/approve"))
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .await;

    approve_response.assert_status(StatusCode::OK);

    let approve_body: serde_json::Value = approve_response.json();

    assert_eq!(approve_body["id"], proposal_id);
    assert_eq!(approve_body["status"], "approved");

    let row = sqlx::query!(
        "SELECT change_status::text FROM changes WHERE id = $1::uuid",
        proposal_id.parse::<uuid::Uuid>().unwrap()
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(row.change_status, Some("approved".to_string()));

    let session = sqlx::query!(
        r#"
        SELECT subject, grp, classroom, duration_min
        FROM sessions
        WHERE subject = 'ALG'
        LIMIT 1
        "#
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(session.subject, "ALG");
    assert_eq!(session.grp, "Teoría");
    assert_eq!(session.classroom, Some("Aula 101".to_string()));
    assert_eq!(session.duration_min, 90);
}

#[tokio::test]
async fn approve_proposal_modify_actualiza_sesion() {
    let ctx = setup().await;

    let professor_token = login_as(&ctx, "prof_modify@uniovi.es", "professor").await;

    let session_id = create_test_session(&ctx.pool).await;

    let response = ctx
        .server
        .post("/proposals")
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .json(&json!({
            "changeType": "modify",
            "changes": {
                "sessionId": session_id,
                "newDuration": 60,
                "newClassroom": "Aula 202"
            }
        }))
        .await;

    response.assert_status(StatusCode::CREATED);

    let body: serde_json::Value = response.json();
    let proposal_id = body["id"].as_str().unwrap();

    let approve_response = ctx
        .server
        .patch(&format!("/proposals/{proposal_id}/approve"))
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .await;

    approve_response.assert_status(StatusCode::OK);

    let session = sqlx::query!(
        r#"
        SELECT duration_min, classroom
        FROM sessions
        WHERE id = $1
        "#,
        session_id
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(session.duration_min, 60);
    assert_eq!(session.classroom, Some("Aula 202".to_string()));
}

#[tokio::test]
async fn approve_proposal_delete_elimina_sesion() {
    let ctx = setup().await;

    let professor_token = login_as(&ctx, "prof_delete@uniovi.es", "professor").await;

    let session_id = create_test_session(&ctx.pool).await;

    let response = ctx
        .server
        .post("/proposals")
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .json(&json!({
            "changeType": "delete",
            "changes": {
                "sessionId": session_id
            }
        }))
        .await;

    response.assert_status(StatusCode::CREATED);

    let body: serde_json::Value = response.json();
    let proposal_id = body["id"].as_str().unwrap();

    let approve_response = ctx
        .server
        .patch(&format!("/proposals/{proposal_id}/approve"))
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .await;

    approve_response.assert_status(StatusCode::OK);

    let deleted = sqlx::query!("SELECT id FROM sessions WHERE id = $1", session_id)
        .fetch_optional(&ctx.pool)
        .await
        .unwrap();

    assert!(deleted.is_none());
}

#[tokio::test]
async fn approve_proposal_devuelve_409_si_ya_esta_aprobada() {
    let ctx = setup().await;

    let professor_token = login_as(&ctx, "prof_conflict@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/proposals")
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .json(&json!({
            "changeType": "create",
            "changes": {
                "subject": "ALG",
                "grp": "Teoría",
                "newStartsAt": "2025-09-15T09:00:00Z",
                "newDuration": 90,
                "newClassroom": "Aula 101"  // añadido: era el campo que faltaba
            }
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    let proposal_id = body["id"].as_str().unwrap();

    ctx.server
        .patch(&format!("/proposals/{proposal_id}/approve"))
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .await
        .assert_status(StatusCode::OK);

    let second_response = ctx
        .server
        .patch(&format!("/proposals/{proposal_id}/approve"))
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .await;

    second_response.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn approve_proposal_devuelve_404_si_no_existe() {
    let ctx = setup().await;

    let professor_token = login_as(&ctx, "prof404@uniovi.es", "professor").await;

    let response = ctx
        .server
        .patch("/proposals/00000000-0000-0000-0000-000000000000/approve")
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn approve_proposal_devuelve_401_sin_autenticacion() {
    let ctx = setup().await;

    // Necesitamos una proposal real: con UUID inexistente el router devuelve 404
    // antes de que el middleware de auth pueda rechazar con 401.
    // Creamos una proposal y usamos su ID para que la ruta sea válida.
    let professor_token = login_as(&ctx, "prof_auth_check@uniovi.es", "professor").await;

    let create_response = ctx
        .server
        .post("/proposals")
        .add_header("Authorization", format!("Bearer {professor_token}"))
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

    create_response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = create_response.json();
    let proposal_id = body["id"].as_str().unwrap();

    let response = ctx
        .server
        .patch(&format!("/proposals/{proposal_id}/approve"))
        // Sin Authorization header
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn approve_proposal_devuelve_403_si_no_es_professor_or_above() {
    let ctx = setup().await;

    // Igual que el 401: necesitamos una proposal real para que el middleware
    // de autorización se ejecute y pueda devolver 403.
    let professor_token = login_as(&ctx, "prof_for_forbidden@uniovi.es", "professor").await;
    let student_token = login_as(&ctx, "student_approve@uniovi.es", "student").await;

    let create_response = ctx
        .server
        .post("/proposals")
        .add_header("Authorization", format!("Bearer {professor_token}"))
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

    create_response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = create_response.json();
    let proposal_id = body["id"].as_str().unwrap();

    let response = ctx
        .server
        .patch(&format!("/proposals/{proposal_id}/approve"))
        .add_header("Authorization", format!("Bearer {student_token}"))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn approve_proposal_notifica_al_autor() {
    let ctx = setup().await;

    let professor_token = login_as(&ctx, "prof_notify_approve@uniovi.es", "professor").await;
    let professor_id: uuid::Uuid = sqlx::query_scalar!(
        "SELECT id FROM users WHERE email = $1",
        "prof_notify_approve@uniovi.es"
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    let response = ctx
        .server
        .post("/proposals")
        .add_header("Authorization", format!("Bearer {professor_token}"))
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

    ctx.server
        .patch(&format!("/proposals/{proposal_id}/approve"))
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .await
        .assert_status(StatusCode::OK);

    let notification = sqlx::query!(
        r#"SELECT type::text AS notif_type, proposal_id FROM notifications
           WHERE user_id = $1 AND type = 'proposal_approved'"#,
        professor_id
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("Debe existir una notificación de propuesta aprobada para el autor");

    assert_eq!(notification.notif_type.as_deref(), Some("proposal_approved"));
    assert_eq!(notification.proposal_id, Some(proposal_id));
}
