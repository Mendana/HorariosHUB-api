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

async fn create_event(ctx: &crate::common::TestContext, token: &str) -> serde_json::Value {
    let response = ctx
        .server
        .post("/events")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "title": "Evento a borrar",
            "subject": "IPS",
            "startTime": "2026-10-01T09:00:00Z",
            "endTime":   "2026-10-01T10:00:00Z"
        }))
        .await;
    response.assert_status(StatusCode::CREATED);
    response.json()
}

#[tokio::test]
async fn delete_events_devuelve_401_sin_autenticar() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_del_setup1@uniovi.es", "professor").await;
    let event = create_event(&ctx, &token).await;
    let id = event["id"].as_str().unwrap();

    let response = ctx.server.delete(&format!("/events/{id}")).await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn delete_events_devuelve_403_como_student() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let prof_token = login_as(&ctx, "prof_ev_del_setup2@uniovi.es", "professor").await;
    let event = create_event(&ctx, &prof_token).await;
    let id = event["id"].as_str().unwrap();

    let student_token = login_as(&ctx, "student_ev_del@uniovi.es", "student").await;

    let response = ctx
        .server
        .delete(&format!("/events/{id}"))
        .add_header("Authorization", format!("Bearer {student_token}"))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn delete_events_devuelve_404_si_no_existe() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_ev_del_404@uniovi.es", "professor").await;

    let response = ctx
        .server
        .delete(&format!("/events/{}", uuid::Uuid::new_v4()))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_events_elimina_el_evento_y_sus_grupos() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_del_ok@uniovi.es", "professor").await;
    let event = create_event(&ctx, &token).await;
    let id: uuid::Uuid = event["id"].as_str().unwrap().parse().unwrap();

    let response = ctx
        .server
        .delete(&format!("/events/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let exists = sqlx::query_scalar!("SELECT COUNT(*) FROM events WHERE id = $1", id)
        .fetch_one(&ctx.pool)
        .await
        .unwrap();
    assert_eq!(exists, Some(0));

    let groups_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM event_groups WHERE event_id = $1",
        id
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    assert_eq!(groups_count, Some(0));
}

#[tokio::test]
async fn delete_events_segunda_vez_devuelve_404() {
    let ctx = setup().await;
    seed_subject(&ctx, "IPS", &["T.1"]).await;
    let token = login_as(&ctx, "prof_ev_del_twice@uniovi.es", "professor").await;
    let event = create_event(&ctx, &token).await;
    let id = event["id"].as_str().unwrap();

    ctx.server
        .delete(&format!("/events/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .assert_status(StatusCode::OK);

    let response = ctx
        .server
        .delete(&format!("/events/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}
