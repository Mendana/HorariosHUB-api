// ── REJECT PROPOSAL ───────────────────────────────────────────────────────────
use crate::common::{create_test_session, login_as, setup};
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn reject_proposal_create_devuelve_200_y_rechaza() {
    let ctx = setup().await;

    let professor_token = login_as(&ctx, "prof_reject@uniovi.es", "professor").await;

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

    let reject_response = ctx
        .server
        .patch(&format!("/proposals/{proposal_id}/reject"))
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .await;

    reject_response.assert_status(StatusCode::OK);

    let reject_body: serde_json::Value = reject_response.json();

    assert_eq!(reject_body["id"], proposal_id);
    assert_eq!(reject_body["status"], "rejected");

    let row = sqlx::query!(
        "SELECT change_status::text FROM changes WHERE id = $1::uuid",
        proposal_id.parse::<uuid::Uuid>().unwrap()
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(row.change_status, Some("rejected".to_string()));
}

#[tokio::test]
async fn reject_proposal_modify_no_modifica_sesion() {
    let ctx = setup().await;

    let professor_token = login_as(&ctx, "prof_reject_modify@uniovi.es", "professor").await;

    let session_id = create_test_session(&ctx.pool).await;

    // Guardamos los valores originales antes de proponer el cambio
    let original = sqlx::query!(
        "SELECT duration_min, classroom FROM sessions WHERE id = $1",
        session_id
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

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

    let reject_response = ctx
        .server
        .patch(&format!("/proposals/{proposal_id}/reject"))
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .await;

    reject_response.assert_status(StatusCode::OK);

    // La sesión no debe haber cambiado
    let session = sqlx::query!(
        "SELECT duration_min, classroom FROM sessions WHERE id = $1",
        session_id
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(session.duration_min, original.duration_min);
    assert_eq!(session.classroom, original.classroom);
}

#[tokio::test]
async fn reject_proposal_delete_no_elimina_sesion() {
    let ctx = setup().await;

    let professor_token = login_as(&ctx, "prof_reject_delete@uniovi.es", "professor").await;

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

    let reject_response = ctx
        .server
        .patch(&format!("/proposals/{proposal_id}/reject"))
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .await;

    reject_response.assert_status(StatusCode::OK);

    // La sesión debe seguir existiendo
    let still_exists = sqlx::query!("SELECT id FROM sessions WHERE id = $1", session_id)
        .fetch_optional(&ctx.pool)
        .await
        .unwrap();

    assert!(still_exists.is_some());
}

#[tokio::test]
async fn reject_proposal_devuelve_409_si_ya_esta_rechazada() {
    let ctx = setup().await;

    let professor_token = login_as(&ctx, "prof_reject_conflict@uniovi.es", "professor").await;

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

    ctx.server
        .patch(&format!("/proposals/{proposal_id}/reject"))
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .await
        .assert_status(StatusCode::OK);

    let second_response = ctx
        .server
        .patch(&format!("/proposals/{proposal_id}/reject"))
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .await;

    second_response.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn reject_proposal_devuelve_409_si_ya_esta_aprobada() {
    let ctx = setup().await;

    let professor_token =
        login_as(&ctx, "prof_reject_already_approved@uniovi.es", "professor").await;

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

    ctx.server
        .patch(&format!("/proposals/{proposal_id}/approve"))
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .await
        .assert_status(StatusCode::OK);

    let reject_response = ctx
        .server
        .patch(&format!("/proposals/{proposal_id}/reject"))
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .await;

    reject_response.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn reject_proposal_devuelve_404_si_no_existe() {
    let ctx = setup().await;

    let professor_token = login_as(&ctx, "prof_reject_404@uniovi.es", "professor").await;

    let response = ctx
        .server
        .patch("/proposals/00000000-0000-0000-0000-000000000000/reject")
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn reject_proposal_devuelve_401_sin_autenticacion() {
    let ctx = setup().await;

    let professor_token = login_as(&ctx, "prof_reject_auth_check@uniovi.es", "professor").await;

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
        .patch(&format!("/proposals/{proposal_id}/reject"))
        // Sin Authorization header
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn reject_proposal_notifica_al_autor() {
    let ctx = setup().await;

    let professor_token = login_as(&ctx, "prof_notify_reject@uniovi.es", "professor").await;
    let professor_id: uuid::Uuid = sqlx::query_scalar!(
        "SELECT id FROM users WHERE email = $1",
        "prof_notify_reject@uniovi.es"
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
        .patch(&format!("/proposals/{proposal_id}/reject"))
        .add_header("Authorization", format!("Bearer {professor_token}"))
        .await
        .assert_status(StatusCode::OK);

    let notification = sqlx::query!(
        r#"SELECT type::text AS notif_type, proposal_id FROM notifications
           WHERE user_id = $1 AND type = 'proposal_rejected'"#,
        professor_id
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("Debe existir una notificación de propuesta rechazada para el autor");

    assert_eq!(notification.notif_type.as_deref(), Some("proposal_rejected"));
    assert_eq!(notification.proposal_id, Some(proposal_id));
}

#[tokio::test]
async fn reject_proposal_devuelve_403_si_no_es_professor_or_above() {
    let ctx = setup().await;

    let professor_token = login_as(&ctx, "prof_for_reject_forbidden@uniovi.es", "professor").await;
    let student_token = login_as(&ctx, "student_reject@uniovi.es", "student").await;

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
        .patch(&format!("/proposals/{proposal_id}/reject"))
        .add_header("Authorization", format!("Bearer {student_token}"))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}
