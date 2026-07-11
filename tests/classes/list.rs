use crate::common::{login_as, setup};
use axum::http::StatusCode;
use chrono::{Duration, TimeZone, Utc};

// Inserta N sesiones con subject, grp, starts_at y classroom opcionales.
// Devuelve los IDs insertados.
async fn insert_session(
    pool: &sqlx::PgPool,
    subject: &str,
    grp: &str,
    starts_at: chrono::DateTime<Utc>,
    duration_min: i32,
    classroom: Option<&str>,
) -> uuid::Uuid {
    sqlx::query!(
        "INSERT INTO subject_groups (subject, grp) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        subject,
        grp
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query_scalar!(
        r#"
        INSERT INTO sessions (subject, grp, starts_at, duration_min, classroom, source, is_overridden)
        VALUES ($1, $2, $3, $4, $5, 'scraper', false)
        RETURNING id
        "#,
        subject,
        grp,
        starts_at,
        duration_min,
        classroom,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn get_classes_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx.server.get("/classes").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_classes_devuelve_200_lista_vacia() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student@uniovi.es", "student").await;

    let response = ctx
        .server
        .get("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 0);
    assert!(body["classes"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn get_classes_devuelve_shape_correcta() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student2@uniovi.es", "student").await;

    let starts_at = Utc::now();
    insert_session(&ctx.pool, "ALG", "Teoría", starts_at, 90, Some("Aula 1")).await;

    let response = ctx
        .server
        .get("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 1);

    let classes = body["classes"].as_array().unwrap();
    assert_eq!(classes.len(), 1);

    let c = &classes[0];
    assert!(c["id"].is_string());
    assert_eq!(c["subject"], "ALG");
    assert_eq!(c["subjectType"], "Teoría");
    assert_eq!(c["classroom"], "Aula 1");
    assert!(c["startTime"].is_string());
    assert!(c["endTime"].is_string());
}

#[tokio::test]
async fn get_classes_classroom_nula_cuando_no_hay() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student3@uniovi.es", "student").await;

    insert_session(&ctx.pool, "CAL", "B", Utc::now(), 60, None).await;

    let response = ctx
        .server
        .get("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let classes = body["classes"].as_array().unwrap();
    assert!(classes[0]["classroom"].is_null());
}

#[tokio::test]
async fn get_classes_filtra_por_search() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student4@uniovi.es", "student").await;

    let now = Utc::now();
    insert_session(&ctx.pool, "ALGEBRA", "A", now, 60, None).await;
    insert_session(&ctx.pool, "CALCULO", "A", now, 60, None).await;

    let response = ctx
        .server
        .get("/classes?search=ALG")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 1);
    assert_eq!(body["classes"][0]["subject"], "ALGEBRA");
}

#[tokio::test]
async fn get_classes_filtra_por_week() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student5@uniovi.es", "student").await;

    // Semana 24 de 2026: lunes 8 jun 2026
    let monday_w24 = chrono::NaiveDate::from_isoywd_opt(2026, 24, chrono::Weekday::Mon).unwrap();
    let in_week = Utc.from_utc_datetime(&monday_w24.and_hms_opt(10, 0, 0).unwrap());
    let out_of_week = in_week + Duration::days(8); // semana 25

    insert_session(&ctx.pool, "FIS", "A", in_week, 60, None).await;
    insert_session(&ctx.pool, "QUI", "A", out_of_week, 60, None).await;

    let response = ctx
        .server
        .get("/classes?week=2026-W24")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 1);
    assert_eq!(body["classes"][0]["subject"], "FIS");
}

#[tokio::test]
async fn get_classes_week_formato_invalido_devuelve_400() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student6@uniovi.es", "student").await;

    let response = ctx
        .server
        .get("/classes?week=2026-24")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_classes_paginacion_funciona() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student7@uniovi.es", "student").await;

    let base = Utc::now();
    for i in 0..5i64 {
        insert_session(&ctx.pool, "PAG", "A", base + Duration::hours(i), 60, None).await;
    }

    // Página 1, limit 2 → 2 resultados, total 5
    let r1 = ctx
        .server
        .get("/classes?limit=2&page=1")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;
    r1.assert_status_ok();
    let b1: serde_json::Value = r1.json();
    assert_eq!(b1["total"], 5);
    assert_eq!(b1["classes"].as_array().unwrap().len(), 2);

    // Página 3, limit 2 → 1 resultado (el último)
    let r3 = ctx
        .server
        .get("/classes?limit=2&page=3")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;
    r3.assert_status_ok();
    let b3: serde_json::Value = r3.json();
    assert_eq!(b3["classes"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn get_classes_ordena_por_nombre_asc() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student8@uniovi.es", "student").await;

    let now = Utc::now();
    insert_session(&ctx.pool, "ZOO", "A", now, 60, None).await;
    insert_session(&ctx.pool, "AAA", "A", now, 60, None).await;

    let response = ctx
        .server
        .get("/classes?sort=name&dir=asc")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let names: Vec<&str> = body["classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["subject"].as_str().unwrap())
        .collect();
    // AAA debe ir antes que ZOO
    let aaa_pos = names.iter().position(|&n| n == "AAA").unwrap();
    let zoo_pos = names.iter().position(|&n| n == "ZOO").unwrap();
    assert!(aaa_pos < zoo_pos);
}

#[tokio::test]
async fn get_classes_ordena_por_nombre_desc() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student9@uniovi.es", "student").await;

    let now = Utc::now();
    insert_session(&ctx.pool, "ZOO", "A", now, 60, None).await;
    insert_session(&ctx.pool, "AAA", "A", now, 60, None).await;

    let response = ctx
        .server
        .get("/classes?sort=name&dir=desc")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let names: Vec<&str> = body["classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["subject"].as_str().unwrap())
        .collect();
    let aaa_pos = names.iter().position(|&n| n == "AAA").unwrap();
    let zoo_pos = names.iter().position(|&n| n == "ZOO").unwrap();
    assert!(zoo_pos < aaa_pos);
}

#[tokio::test]
async fn get_classes_end_time_es_starts_at_mas_duracion() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student10@uniovi.es", "student").await;

    let starts_at = Utc.from_utc_datetime(
        &chrono::NaiveDate::from_ymd_opt(2026, 6, 14)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap(),
    );

    insert_session(&ctx.pool, "TSTEND", "A", starts_at, 90, None).await;

    let response = ctx
        .server
        .get("/classes")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let c = &body["classes"][0];

    let start: chrono::DateTime<Utc> = c["startTime"].as_str().unwrap().parse().unwrap();
    let end: chrono::DateTime<Utc> = c["endTime"].as_str().unwrap().parse().unwrap();
    assert_eq!((end - start).num_minutes(), 90);
}
