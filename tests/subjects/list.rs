// GET /subjects/list
use crate::common::setup;
use axum::http::StatusCode;

async fn insert_group(pool: &sqlx::PgPool, subject: &str, grp: &str) {
    sqlx::query!(
        "INSERT INTO subject_groups (subject, grp) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        subject,
        grp
    )
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn get_subjects_list_devuelve_200_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx.server.get("/subjects/list").await;

    response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn get_subjects_list_devuelve_lista_vacia_si_no_hay_asignaturas() {
    let ctx = setup().await;

    let body: serde_json::Value = ctx.server.get("/subjects/list").await.json();

    assert_eq!(body["subjects"], serde_json::json!([]));
}

#[tokio::test]
async fn get_subjects_list_devuelve_codigos_de_asignaturas() {
    let ctx = setup().await;

    insert_group(&ctx.pool, "ALG", "Teoría").await;
    insert_group(&ctx.pool, "CDI", "Práctica").await;

    let body: serde_json::Value = ctx.server.get("/subjects/list").await.json();
    let subjects: Vec<&str> = body["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();

    assert!(subjects.contains(&"ALG"));
    assert!(subjects.contains(&"CDI"));
}

#[tokio::test]
async fn get_subjects_list_no_repite_asignaturas_con_varios_grupos() {
    let ctx = setup().await;

    insert_group(&ctx.pool, "UNIQ", "Teoría").await;
    insert_group(&ctx.pool, "UNIQ", "Práctica").await;

    let body: serde_json::Value = ctx.server.get("/subjects/list").await.json();
    let subjects: Vec<&str> = body["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();

    assert_eq!(subjects.iter().filter(|&&s| s == "UNIQ").count(), 1);
}

#[tokio::test]
async fn get_subjects_list_elementos_son_strings() {
    let ctx = setup().await;

    insert_group(&ctx.pool, "FIS", "A").await;

    let body: serde_json::Value = ctx.server.get("/subjects/list").await.json();
    let subjects = body["subjects"].as_array().unwrap();

    assert!(subjects.iter().all(|v| v.is_string()));
}

#[tokio::test]
async fn get_subjects_list_ordenados_alfabeticamente() {
    let ctx = setup().await;

    insert_group(&ctx.pool, "ZZZ", "A").await;
    insert_group(&ctx.pool, "AAA", "A").await;

    let body: serde_json::Value = ctx.server.get("/subjects/list").await.json();
    let subjects: Vec<&str> = body["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();

    let aaa = subjects.iter().position(|&s| s == "AAA").unwrap();
    let zzz = subjects.iter().position(|&s| s == "ZZZ").unwrap();
    assert!(aaa < zzz);
}
