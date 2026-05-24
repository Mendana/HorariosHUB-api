// ── POST /subjects/selection ───────────────────────────────────────────────────
use crate::common::{login_as, setup};
use axum::http::StatusCode;
use uuid::Uuid;

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Inserta una asignatura+grupo y devuelve su UUID.
async fn insertar_grupo(ctx: &crate::common::TestContext, subject: &str, grp: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"
        INSERT INTO subject_groups (subject, grp)
        VALUES ($1, $2)
        ON CONFLICT (subject, grp) DO UPDATE SET subject = EXCLUDED.subject
        RETURNING id
        "#,
        subject,
        grp
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap()
}

/// Devuelve los pares (subject, grp) que tiene el usuario en schedule.
async fn grupos_en_schedule(
    ctx: &crate::common::TestContext,
    email: &str,
) -> Vec<(String, String)> {
    sqlx::query!(
        r#"
        SELECT s.subject, s.grp
        FROM schedule s
        JOIN users u ON u.id = s.user_id
        WHERE u.email = $1
        ORDER BY s.subject, s.grp
        "#,
        email
    )
    .fetch_all(&ctx.pool)
    .await
    .unwrap()
    .into_iter()
    .map(|r| (r.subject, r.grp))
    .collect()
}

// ─── Autenticación ────────────────────────────────────────────────────────────

#[tokio::test]
async fn post_selection_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/subjects/selection")
        .json(&serde_json::json!({ "groups": [] }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn post_selection_devuelve_200_como_student() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student_sel@uniovi.es", "student").await;

    let response = ctx
        .server
        .post("/subjects/selection")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "groups": [] }))
        .await;

    response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn post_selection_devuelve_200_como_profesor() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_sel@uniovi.es", "professor").await;

    let response = ctx
        .server
        .post("/subjects/selection")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "groups": [] }))
        .await;

    response.assert_status(StatusCode::OK);
}

// ─── Respuesta ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn post_selection_respuesta_tiene_message_y_count() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_resp_sel@uniovi.es", "professor").await;
    let id = insertar_grupo(&ctx, "ALG", "Teoría").await;

    let body = ctx
        .server
        .post("/subjects/selection")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "groups": [id] }))
        .await
        .json::<serde_json::Value>();

    assert!(body["message"].is_string(), "falta campo 'message'");
    assert!(body["count"].is_number(), "falta campo 'count'");
}

#[tokio::test]
async fn post_selection_count_coincide_con_grupos_enviados() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_count_sel@uniovi.es", "professor").await;
    let id1 = insertar_grupo(&ctx, "MAT", "Teoría").await;
    let id2 = insertar_grupo(&ctx, "FIS", "Laboratorio").await;
    let id3 = insertar_grupo(&ctx, "QUI", "Grupo A").await;

    let body = ctx
        .server
        .post("/subjects/selection")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "groups": [id1, id2, id3] }))
        .await
        .json::<serde_json::Value>();

    assert_eq!(body["count"], 3);
    assert_eq!(body["message"], "Selección guardada");
}

// ─── Semántica de reemplazo ───────────────────────────────────────────────────

#[tokio::test]
async fn post_selection_guarda_grupos_en_schedule() {
    let ctx = setup().await;
    let email = "prof_save_sel@uniovi.es";
    let token = login_as(&ctx, email, "professor").await;
    let id = insertar_grupo(&ctx, "ECO", "Teoría").await;

    ctx.server
        .post("/subjects/selection")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "groups": [id] }))
        .await;

    let grupos = grupos_en_schedule(&ctx, email).await;
    assert_eq!(grupos, vec![("ECO".to_string(), "Teoría".to_string())]);
}

#[tokio::test]
async fn post_selection_reemplaza_seleccion_previa() {
    let ctx = setup().await;
    let email = "prof_replace_sel@uniovi.es";
    let token = login_as(&ctx, email, "professor").await;

    let id_alg = insertar_grupo(&ctx, "ALG2", "Teoría").await;
    let id_cdi = insertar_grupo(&ctx, "CDI", "Laboratorio").await;
    let id_prog = insertar_grupo(&ctx, "PROG", "Grupo A").await;

    // Primera selección: ALG + CDI
    ctx.server
        .post("/subjects/selection")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "groups": [id_alg, id_cdi] }))
        .await;

    // Segunda selección: solo PROG (reemplaza completamente)
    ctx.server
        .post("/subjects/selection")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "groups": [id_prog] }))
        .await;

    let grupos = grupos_en_schedule(&ctx, email).await;
    assert_eq!(
        grupos,
        vec![("PROG".to_string(), "Grupo A".to_string())],
        "solo debe quedar PROG/Grupo A tras el reemplazo"
    );
}

#[tokio::test]
async fn post_selection_con_lista_vacia_borra_todo() {
    let ctx = setup().await;
    let email = "prof_clear_sel@uniovi.es";
    let token = login_as(&ctx, email, "professor").await;

    let id = insertar_grupo(&ctx, "BIO", "Teoría").await;

    // Seleccionamos algo
    ctx.server
        .post("/subjects/selection")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "groups": [id] }))
        .await;

    // Vaciamos la selección
    ctx.server
        .post("/subjects/selection")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "groups": [] }))
        .await;

    let grupos = grupos_en_schedule(&ctx, email).await;
    assert!(grupos.is_empty(), "la selección debe quedar vacía");
}

// ─── Aislamiento entre usuarios ───────────────────────────────────────────────

#[tokio::test]
async fn post_selection_es_independiente_por_usuario() {
    let ctx = setup().await;
    let email_a = "user_a_sel@uniovi.es";
    let email_b = "user_b_sel@uniovi.es";
    let token_a = login_as(&ctx, email_a, "student").await;
    let token_b = login_as(&ctx, email_b, "student").await;

    let id_geo = insertar_grupo(&ctx, "GEO", "Teoría").await;
    let id_his = insertar_grupo(&ctx, "HIS", "Grupo A").await;

    // A selecciona GEO, B selecciona HIS
    ctx.server
        .post("/subjects/selection")
        .add_header("Authorization", format!("Bearer {token_a}"))
        .json(&serde_json::json!({ "groups": [id_geo] }))
        .await;

    ctx.server
        .post("/subjects/selection")
        .add_header("Authorization", format!("Bearer {token_b}"))
        .json(&serde_json::json!({ "groups": [id_his] }))
        .await;

    let grupos_a = grupos_en_schedule(&ctx, email_a).await;
    let grupos_b = grupos_en_schedule(&ctx, email_b).await;

    assert_eq!(grupos_a, vec![("GEO".to_string(), "Teoría".to_string())]);
    assert_eq!(grupos_b, vec![("HIS".to_string(), "Grupo A".to_string())]);
}

// ─── Integración con /subjects/catalog ───────────────────────────────────────

#[tokio::test]
async fn post_selection_se_refleja_en_catalog() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_catalog_sel@uniovi.es", "professor").await;

    let id = insertar_grupo(&ctx, "INF", "Laboratorio").await;

    ctx.server
        .post("/subjects/selection")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "groups": [id] }))
        .await;

    let catalog = ctx
        .server
        .get("/subjects/catalog")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    let grupo = catalog["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["code"] == "INF")
        .unwrap()["groups"][0]
        .clone();

    assert_eq!(
        grupo["selected"], true,
        "INF/Laboratorio debe aparecer como selected en catalog"
    );
}

#[tokio::test]
async fn post_selection_borra_refleja_unselected_en_catalog() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_uncatalog_sel@uniovi.es", "professor").await;

    let id = insertar_grupo(&ctx, "DER", "Teoría").await;

    // Seleccionamos
    ctx.server
        .post("/subjects/selection")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "groups": [id] }))
        .await;

    // Vaciamos
    ctx.server
        .post("/subjects/selection")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "groups": [] }))
        .await;

    let catalog = ctx
        .server
        .get("/subjects/catalog")
        .add_header("Authorization", format!("Bearer {token}"))
        .await
        .json::<serde_json::Value>();

    let grupo = catalog["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["code"] == "DER")
        .unwrap()["groups"][0]
        .clone();

    assert_eq!(
        grupo["selected"], false,
        "DER/Teoría debe aparecer como no selected tras vaciado"
    );
}
