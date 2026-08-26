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

#[tokio::test]
async fn post_proposals_notifica_a_profesores_y_admins_pero_no_al_autor_student() {
    let ctx = setup().await;

    let professor_token = login_as(&ctx, "prof_reviewer@uniovi.es", "professor").await;
    let professor_id: uuid::Uuid = sqlx::query_scalar!(
        "SELECT id FROM users WHERE email = $1",
        "prof_reviewer@uniovi.es"
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    let admin_token = login_as(&ctx, "admin_reviewer@uniovi.es", "admin").await;
    let admin_id: uuid::Uuid = sqlx::query_scalar!(
        "SELECT id FROM users WHERE email = $1",
        "admin_reviewer@uniovi.es"
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    let _ = (professor_token, admin_token);

    let student_token = login_as(&ctx, "student_proposer@uniovi.es", "student").await;
    let student_id: uuid::Uuid = sqlx::query_scalar!(
        "SELECT id FROM users WHERE email = $1",
        "student_proposer@uniovi.es"
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    let response = ctx
        .server
        .post("/proposals")
        .add_header("Authorization", format!("Bearer {student_token}"))
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

    for reviewer_id in [professor_id, admin_id] {
        let notification = sqlx::query!(
            r#"SELECT type::text AS notif_type FROM notifications
               WHERE user_id = $1 AND proposal_id = $2 AND type = 'proposal_created'"#,
            reviewer_id,
            proposal_id
        )
        .fetch_one(&ctx.pool)
        .await
        .expect("Debe existir una notificación de propuesta nueva para el revisor");

        assert_eq!(notification.notif_type.as_deref(), Some("proposal_created"));
    }

    // El estudiante que crea la propuesta no es profesor/admin: no se le notifica
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1",
        student_id
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    assert_eq!(count, Some(0));
}
