// GET /subjects/{code}/groups
use crate::common::setup;
use axum::http::StatusCode;
use uuid::Uuid;

#[tokio::test]
async fn get_groups_devuelve_404_si_asignatura_no_existe() {
    let ctx = setup().await;

    let response = ctx.server.get("/subjects/NOEXISTE/groups").await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_groups_devuelve_200_con_grupos_existentes() {
    let ctx = setup().await;

    sqlx::query!(
        "INSERT INTO subject_groups (subject, grp) VALUES ('GTEST', 'Teoría') ON CONFLICT DO NOTHING"
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    let response = ctx.server.get("/subjects/GTEST/groups").await;

    response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn get_groups_devuelve_shape_correcta() {
    let ctx = setup().await;

    sqlx::query!(
        "INSERT INTO subject_groups (subject, grp) VALUES ('SHAPE', 'GL1') ON CONFLICT DO NOTHING"
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    let body: serde_json::Value = ctx.server.get("/subjects/SHAPE/groups").await.json();
    let groups = body["groups"].as_array().unwrap();

    assert!(!groups.is_empty());
    let g = &groups[0];
    assert!(g["id"].as_str().is_some(), "id debe ser string UUID");
    assert!(g["name"].as_str().is_some(), "name debe ser string");
    assert!(
        Uuid::parse_str(g["id"].as_str().unwrap()).is_ok(),
        "id debe ser UUID válido"
    );
}

#[tokio::test]
async fn get_groups_devuelve_todos_los_grupos_de_la_asignatura() {
    let ctx = setup().await;

    for grp in ["T1", "T2", "P1"] {
        sqlx::query!(
            "INSERT INTO subject_groups (subject, grp) VALUES ('MULTI', $1) ON CONFLICT DO NOTHING",
            grp
        )
        .execute(&ctx.pool)
        .await
        .unwrap();
    }

    let body: serde_json::Value = ctx.server.get("/subjects/MULTI/groups").await.json();
    let groups = body["groups"].as_array().unwrap();

    assert_eq!(groups.len(), 3);
}

#[tokio::test]
async fn get_groups_no_mezcla_grupos_de_otras_asignaturas() {
    let ctx = setup().await;

    sqlx::query!(
        "INSERT INTO subject_groups (subject, grp) VALUES ('ISOLA', 'A') ON CONFLICT DO NOTHING"
    )
    .execute(&ctx.pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO subject_groups (subject, grp) VALUES ('ISOLB', 'A') ON CONFLICT DO NOTHING"
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    let body: serde_json::Value = ctx.server.get("/subjects/ISOLA/groups").await.json();
    let groups = body["groups"].as_array().unwrap();

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["name"].as_str().unwrap(), "A");
}

#[tokio::test]
async fn get_groups_id_puede_usarse_como_group_id_en_post_classes() {
    let ctx = setup().await;
    let token = crate::common::login_as(&ctx, "prof_groups@uniovi.es", "professor").await;

    sqlx::query!(
        "INSERT INTO subject_groups (subject, grp) VALUES ('GIDTEST', 'GL1') ON CONFLICT DO NOTHING"
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    let body: serde_json::Value = ctx.server.get("/subjects/GIDTEST/groups").await.json();
    let group_id = body["groups"][0]["id"].as_str().unwrap();

    let response = ctx
        .server
        .post("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "groupId": group_id,
            "startTime": "2025-09-15T09:00:00Z",
            "endTime": "2025-09-15T10:30:00Z"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);

    let class: serde_json::Value = response.json();
    assert_eq!(class["subject"].as_str().unwrap(), "GIDTEST");
    assert_eq!(class["subjectType"].as_str().unwrap(), "GL1");
}
