// ── GET /subjects/catalog ─────────────────────────────────────────────────────
use crate::common::{login_as, setup};
use axum::http::StatusCode;

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Inserta una asignatura+grupo en subject_groups (ON CONFLICT DO NOTHING).
async fn insertar_grupo(ctx: &crate::common::TestContext, subject: &str, grp: &str) {
    sqlx::query!(
        "INSERT INTO subject_groups (subject, grp) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        subject,
        grp
    )
    .execute(&ctx.pool)
    .await
    .unwrap();
}

/// Suscribe al usuario (por email) al grupo (subject, grp) en la tabla schedule.
async fn seleccionar_grupo(
    ctx: &crate::common::TestContext,
    email: &str,
    subject: &str,
    grp: &str,
) {
    sqlx::query!(
        r#"
        INSERT INTO schedule (user_id, subject, grp)
        SELECT id, $2, $3 FROM users WHERE email = $1
        ON CONFLICT DO NOTHING
        "#,
        email,
        subject,
        grp
    )
    .execute(&ctx.pool)
    .await
    .unwrap();
}

// ─── Autenticación ────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_catalog_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx.server.get("/subjects/catalog").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_catalog_devuelve_200_como_student() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student_catalog@uniovi.es", "student").await;

    let response = ctx
        .server
        .get("/subjects/catalog")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    // No se requiere rol mínimo — cualquier usuario autenticado puede acceder
    response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn get_catalog_devuelve_200_como_profesor() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_catalog@uniovi.es", "professor").await;

    let response = ctx
        .server
        .get("/subjects/catalog")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
}

// ─── Respuesta cuando no hay asignaturas ──────────────────────────────────────

#[tokio::test]
async fn get_catalog_sin_asignaturas_devuelve_lista_vacia() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_empty_cat@uniovi.es", "professor").await;

    let body = ctx
        .server
        .get("/subjects/catalog")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    assert_eq!(body["subjects"], serde_json::json!([]));
}

// ─── Estructura de la respuesta ───────────────────────────────────────────────

#[tokio::test]
async fn get_catalog_respuesta_tiene_estructura_correcta() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_struct_cat@uniovi.es", "professor").await;

    insertar_grupo(&ctx, "ALG", "Teoría").await;

    let body = ctx
        .server
        .get("/subjects/catalog")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    assert!(body["subjects"].is_array(), "falta campo 'subjects'");

    let subject = &body["subjects"][0];
    assert!(subject["code"].is_string(), "falta campo 'code'");
    assert!(subject["groups"].is_array(), "falta campo 'groups'");

    let group = &subject["groups"][0];
    assert!(group["id"].is_string(), "falta campo 'id'");
    assert!(group["name"].is_string(), "falta campo 'name'");
    assert!(group["selected"].is_boolean(), "falta campo 'selected'");
}

#[tokio::test]
async fn get_catalog_code_y_name_son_correctos() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_names_cat@uniovi.es", "professor").await;

    insertar_grupo(&ctx, "ALG", "Grupo A").await;

    let body = ctx
        .server
        .get("/subjects/catalog")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    let subject = &body["subjects"][0];
    assert_eq!(subject["code"], "ALG");
    assert_eq!(subject["groups"][0]["name"], "Grupo A");
}

// ─── Campo `selected` ─────────────────────────────────────────────────────────

#[tokio::test]
async fn get_catalog_selected_false_si_no_esta_suscrito() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_unsel_cat@uniovi.es", "professor").await;

    insertar_grupo(&ctx, "MAT", "Teoría").await;
    // No insertamos nada en schedule

    let body = ctx
        .server
        .get("/subjects/catalog")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    let group = body["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["code"] == "MAT")
        .unwrap()["groups"][0]
        .clone();

    assert_eq!(group["selected"], false);
}

#[tokio::test]
async fn get_catalog_selected_true_si_esta_suscrito() {
    let ctx = setup().await;
    let email = "prof_sel_cat@uniovi.es";
    let token = login_as(&ctx, email, "professor").await;

    insertar_grupo(&ctx, "FIS", "Laboratorio").await;
    seleccionar_grupo(&ctx, email, "FIS", "Laboratorio").await;

    let body = ctx
        .server
        .get("/subjects/catalog")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    let group = body["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["code"] == "FIS")
        .unwrap()["groups"][0]
        .clone();

    assert_eq!(group["selected"], true);
}

#[tokio::test]
async fn get_catalog_selected_solo_para_grupos_suscritos() {
    let ctx = setup().await;
    let email = "prof_mixed_cat@uniovi.es";
    let token = login_as(&ctx, email, "professor").await;

    insertar_grupo(&ctx, "QUI", "Teoría").await;
    insertar_grupo(&ctx, "QUI", "Laboratorio").await;

    // Solo suscribimos "Teoría", no "Laboratorio"
    seleccionar_grupo(&ctx, email, "QUI", "Teoría").await;

    let body = ctx
        .server
        .get("/subjects/catalog")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    let groups = body["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["code"] == "QUI")
        .unwrap()["groups"]
        .as_array()
        .unwrap()
        .clone();

    let teoria = groups.iter().find(|g| g["name"] == "Teoría").unwrap();
    let lab = groups.iter().find(|g| g["name"] == "Laboratorio").unwrap();

    assert_eq!(teoria["selected"], true, "Teoría debe estar selected");
    assert_eq!(lab["selected"], false, "Laboratorio no debe estar selected");
}

// ─── Aislamiento entre usuarios ───────────────────────────────────────────────

#[tokio::test]
async fn get_catalog_selected_es_independiente_por_usuario() {
    let ctx = setup().await;
    let email_a = "user_a_cat@uniovi.es";
    let email_b = "user_b_cat@uniovi.es";
    let token_a = login_as(&ctx, email_a, "professor").await;
    let token_b = login_as(&ctx, email_b, "professor").await;

    insertar_grupo(&ctx, "ECO", "Teoría").await;

    // Solo A se suscribe
    seleccionar_grupo(&ctx, email_a, "ECO", "Teoría").await;

    let catalogo_a = ctx
        .server
        .get("/subjects/catalog")
        .add_header("Authorization", format!("Bearer {token_a}"))
        .await
        .json::<serde_json::Value>();

    let catalogo_b = ctx
        .server
        .get("/subjects/catalog")
        .add_header("Authorization", format!("Bearer {token_b}"))
        .await
        .json::<serde_json::Value>();

    let grupo_a = catalogo_a["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["code"] == "ECO")
        .unwrap()["groups"][0]
        .clone();

    let grupo_b = catalogo_b["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["code"] == "ECO")
        .unwrap()["groups"][0]
        .clone();

    assert_eq!(grupo_a["selected"], true, "A suscribió ECO/Teoría");
    assert_eq!(grupo_b["selected"], false, "B no suscribió ECO/Teoría");
}

// ─── Agrupación por asignatura ────────────────────────────────────────────────

#[tokio::test]
async fn get_catalog_grupos_agrupados_por_asignatura() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_group_cat@uniovi.es", "professor").await;

    insertar_grupo(&ctx, "BIO", "Teoría").await;
    insertar_grupo(&ctx, "BIO", "Laboratorio").await;
    insertar_grupo(&ctx, "GEO", "Teoría").await;

    let body = ctx
        .server
        .get("/subjects/catalog")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    let subjects = body["subjects"].as_array().unwrap();

    let bio = subjects.iter().find(|s| s["code"] == "BIO").unwrap();
    let geo = subjects.iter().find(|s| s["code"] == "GEO").unwrap();

    // BIO tiene 2 grupos, GEO tiene 1
    assert_eq!(
        bio["groups"].as_array().unwrap().len(),
        2,
        "BIO debe tener 2 grupos"
    );
    assert_eq!(
        geo["groups"].as_array().unwrap().len(),
        1,
        "GEO debe tener 1 grupo"
    );
}
