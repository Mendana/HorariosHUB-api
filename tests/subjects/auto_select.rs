// ── POST /subjects/auto-select ─────────────────────────────────────────────────
use crate::common::{login_as, setup, setup_with_scraper_url};
use axum::http::StatusCode;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path_regex},
};

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Extrae el `job_id` del cuerpo de una respuesta 202.
fn extraer_job_id(body: &serde_json::Value) -> uuid::Uuid {
    body["job_id"]
        .as_str()
        .expect("falta campo 'job_id' en la respuesta")
        .parse()
        .expect("job_id no es un UUID válido")
}

/// Espera (polling cada 100 ms) hasta que el job alcance el `expected_status`.
/// Falla si no lo alcanza en ~2 segundos.
async fn esperar_status_job(pool: &sqlx::PgPool, job_id: uuid::Uuid, expected_status: &str) {
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let status: Option<String> = sqlx::query_scalar!(
            "SELECT status::text FROM auto_select_jobs WHERE id = $1",
            job_id
        )
        .fetch_one(pool)
        .await
        .unwrap();

        if status.as_deref() == Some(expected_status) {
            return;
        }
    }

    panic!("Timeout: el job {job_id} no alcanzó el status '{expected_status}' en 2 s");
}

/// Devuelve los pares (subject, grp) del usuario en la tabla `schedule`.
async fn grupos_en_schedule(pool: &sqlx::PgPool, email: &str) -> Vec<(String, String)> {
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
    .fetch_all(pool)
    .await
    .unwrap()
    .into_iter()
    .map(|r| (r.subject, r.grp))
    .collect()
}

// ─── Autenticación ────────────────────────────────────────────────────────────

#[tokio::test]
async fn auto_select_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx.server.post("/subjects/auto-select").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

// ─── Validación del formato UO ────────────────────────────────────────────────

#[tokio::test]
async fn auto_select_devuelve_400_para_email_no_uo() {
    let ctx = setup().await;
    let token = login_as(&ctx, "infmatprimero@uniovi.es", "student").await;

    let response = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn auto_select_devuelve_400_para_uo_con_pocos_digitos() {
    let ctx = setup().await;
    // "uo" + 3 dígitos: inválido (mínimo 4)
    let token = login_as(&ctx, "uo123@uniovi.es", "student").await;

    let response = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn auto_select_devuelve_400_para_uo_con_muchos_digitos() {
    let ctx = setup().await;
    // "uo" + 7 dígitos: inválido (máximo 6)
    let token = login_as(&ctx, "uo1234567@uniovi.es", "student").await;

    let response = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn auto_select_devuelve_400_para_uo_con_letras_en_digitos() {
    let ctx = setup().await;
    let token = login_as(&ctx, "uo123abc@uniovi.es", "student").await;

    let response = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

// ─── Respuesta 202 y estructura ───────────────────────────────────────────────

#[tokio::test]
async fn auto_select_devuelve_202_para_usuario_uo_valido() {
    let ctx = setup().await;
    let token = login_as(&ctx, "uo123456@uniovi.es", "student").await;

    let response = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::ACCEPTED);

    let body = response.json::<serde_json::Value>();
    assert!(body["job_id"].is_string(), "falta campo 'job_id'");
    assert_eq!(
        body["status"], "processing",
        "status inicial debe ser 'processing'"
    );
}

#[tokio::test]
async fn auto_select_acepta_uo_con_4_digitos() {
    let ctx = setup().await;
    let token = login_as(&ctx, "uo1234@uniovi.es", "student").await;

    let response = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::ACCEPTED);
}

#[tokio::test]
async fn auto_select_acepta_uo_con_6_digitos() {
    let ctx = setup().await;
    let token = login_as(&ctx, "uo654321@uniovi.es", "student").await;

    let response = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::ACCEPTED);
}

// ─── Job creado en BD ─────────────────────────────────────────────────────────

#[tokio::test]
async fn auto_select_crea_job_en_bd_con_status_processing() {
    let ctx = setup().await;
    let token = login_as(&ctx, "uo111333@uniovi.es", "student").await;

    let response = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    let job_id = extraer_job_id(&response.json::<serde_json::Value>());

    let status: Option<String> = sqlx::query_scalar!(
        "SELECT status::text FROM auto_select_jobs WHERE id = $1",
        job_id
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(
        status.as_deref(),
        Some("processing"),
        "el job debe crearse con status 'processing'"
    );
}

#[tokio::test]
async fn auto_select_job_en_bd_tiene_user_id_correcto() {
    let ctx = setup().await;
    let email = "uo222444@uniovi.es";
    let token = login_as(&ctx, email, "student").await;

    let response = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    let job_id = extraer_job_id(&response.json::<serde_json::Value>());

    let count: Option<i64> = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*)
        FROM auto_select_jobs j
        JOIN users u ON u.id = j.user_id
        WHERE j.id = $1 AND u.email = $2
        "#,
        job_id,
        email
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(
        count,
        Some(1),
        "el job debe estar vinculado al usuario correcto"
    );
}

// ─── Conflicto: job ya activo ─────────────────────────────────────────────────

#[tokio::test]
async fn auto_select_devuelve_409_si_ya_hay_job_activo() {
    let ctx = setup().await;
    let email = "uo333666@uniovi.es";
    let token = login_as(&ctx, email, "student").await;

    // Insertar directamente un job en estado 'processing'
    sqlx::query!(
        r#"
        INSERT INTO auto_select_jobs (user_id, status)
        SELECT id, 'processing'::auto_select_job_status
        FROM users
        WHERE email = $1
        "#,
        email
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    let response = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn auto_select_permite_nuevo_job_tras_completar_el_anterior() {
    let ctx = setup().await;
    let email = "uo444777@uniovi.es";
    let token = login_as(&ctx, email, "student").await;

    // Insertar un job ya completado (no debe bloquear uno nuevo)
    sqlx::query!(
        r#"
        INSERT INTO auto_select_jobs (user_id, status, finished_at)
        SELECT id, 'completed'::auto_select_job_status, NOW()
        FROM users
        WHERE email = $1
        "#,
        email
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    let response = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    // Un job completado no impide lanzar uno nuevo
    response.assert_status(StatusCode::ACCEPTED);
}

// ─── Background: scraper devuelve éxito ──────────────────────────────────────

#[tokio::test]
async fn auto_select_background_actualiza_schedule_del_usuario() {
    let mock_server = MockServer::start().await;

    // El scraper devuelve dos asignaturas
    Mock::given(method("POST"))
        .and(path_regex(r"^/auto-select/uo\d{4,6}$"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ALG,T.1\nFIS,P.2\n"))
        .mount(&mock_server)
        .await;

    let ctx = setup_with_scraper_url(&mock_server.uri()).await;
    let email = "uo333555@uniovi.es";
    let token = login_as(&ctx, email, "student").await;

    // Insertar los grupos en el catálogo para evitar conflictos de FK
    sqlx::query!(
        "INSERT INTO subject_groups (subject, grp) VALUES ('ALG', 'T.1'), ('FIS', 'P.2') ON CONFLICT DO NOTHING"
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    let response = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    let job_id = extraer_job_id(&response.json::<serde_json::Value>());

    esperar_status_job(&ctx.pool, job_id, "completed").await;

    let grupos = grupos_en_schedule(&ctx.pool, email).await;
    assert_eq!(
        grupos,
        vec![
            ("ALG".to_string(), "T.1".to_string()),
            ("FIS".to_string(), "P.2".to_string()),
        ],
        "el schedule debe contener exactamente los grupos del scraper"
    );
}

#[tokio::test]
async fn auto_select_background_marca_job_como_completed_con_conteo() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path_regex(r"^/auto-select/uo\d{4,6}$"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ALG,T.1\nFIS,P.2\nQUI,L.1\n"))
        .mount(&mock_server)
        .await;

    let ctx = setup_with_scraper_url(&mock_server.uri()).await;
    let token = login_as(&ctx, "uo444666@uniovi.es", "student").await;

    sqlx::query!(
        "INSERT INTO subject_groups (subject, grp) VALUES ('ALG', 'T.1'), ('FIS', 'P.2'), ('QUI', 'L.1') ON CONFLICT DO NOTHING"
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    let response = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    let job_id = extraer_job_id(&response.json::<serde_json::Value>());

    esperar_status_job(&ctx.pool, job_id, "completed").await;

    let row = sqlx::query!(
        "SELECT status::text AS status, groups_selected, finished_at FROM auto_select_jobs WHERE id = $1",
        job_id
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(row.status.as_deref(), Some("completed"));
    assert_eq!(
        row.groups_selected,
        Some(3),
        "groups_selected debe reflejar los grupos del scraper"
    );
    assert!(
        row.finished_at.is_some(),
        "finished_at debe quedar establecido"
    );
}

// ─── Background: scraper devuelve error ──────────────────────────────────────

#[tokio::test]
async fn auto_select_background_marca_job_como_failed_si_scraper_responde_500() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path_regex(r"^/auto-select/uo\d{4,6}$"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal scraper error"))
        .mount(&mock_server)
        .await;

    let ctx = setup_with_scraper_url(&mock_server.uri()).await;
    let token = login_as(&ctx, "uo555777@uniovi.es", "student").await;

    let response = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::ACCEPTED);

    let job_id = extraer_job_id(&response.json::<serde_json::Value>());

    esperar_status_job(&ctx.pool, job_id, "failed").await;

    let row = sqlx::query!(
        "SELECT status::text AS status, error FROM auto_select_jobs WHERE id = $1",
        job_id
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();

    assert_eq!(row.status.as_deref(), Some("failed"));
    assert!(
        row.error.is_some(),
        "debe guardarse un mensaje de error cuando el scraper falla"
    );
}

#[tokio::test]
async fn auto_select_background_marca_job_como_failed_si_scraper_no_responde() {
    // Ningún mock registrado → la conexión al mock_server fallará
    let mock_server = MockServer::start().await;
    // Sin mocks: wiremock devuelve 404 a peticiones no registradas
    // Eso hace que el scraper responda con error HTTP → job failed

    let ctx = setup_with_scraper_url(&mock_server.uri()).await;
    let token = login_as(&ctx, "uo666888@uniovi.es", "student").await;

    let response = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    let job_id = extraer_job_id(&response.json::<serde_json::Value>());

    esperar_status_job(&ctx.pool, job_id, "failed").await;
}

// ─── Background: reemplaza schedule previo ───────────────────────────────────

#[tokio::test]
async fn auto_select_background_reemplaza_schedule_existente() {
    let mock_server = MockServer::start().await;

    // El scraper devuelve solo PROG/T.1 (MAT/T.1 no está)
    Mock::given(method("POST"))
        .and(path_regex(r"^/auto-select/uo\d{4,6}$"))
        .respond_with(ResponseTemplate::new(200).set_body_string("PROG,T.1\n"))
        .mount(&mock_server)
        .await;

    let ctx = setup_with_scraper_url(&mock_server.uri()).await;
    let email = "uo777999@uniovi.es";
    let token = login_as(&ctx, email, "student").await;

    sqlx::query!(
        "INSERT INTO subject_groups (subject, grp) VALUES ('MAT', 'T.1'), ('PROG', 'T.1') ON CONFLICT DO NOTHING"
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    // Selección previa: MAT/T.1
    sqlx::query!(
        "INSERT INTO schedule (user_id, subject, grp) SELECT id, 'MAT', 'T.1' FROM users WHERE email = $1",
        email
    )
    .execute(&ctx.pool)
    .await
    .unwrap();

    let response = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    let job_id = extraer_job_id(&response.json::<serde_json::Value>());

    esperar_status_job(&ctx.pool, job_id, "completed").await;

    let grupos = grupos_en_schedule(&ctx.pool, email).await;

    // MAT debe haber desaparecido; PROG es el único que queda
    assert_eq!(
        grupos,
        vec![("PROG".to_string(), "T.1".to_string())],
        "el auto-select debe reemplazar completamente la selección previa"
    );
}

// ─── Aislamiento entre usuarios ───────────────────────────────────────────────

#[tokio::test]
async fn auto_select_los_jobs_son_independientes_por_usuario() {
    let ctx = setup().await;
    let email_a = "uo100200@uniovi.es";
    let email_b = "uo300400@uniovi.es";
    let token_a = login_as(&ctx, email_a, "student").await;
    let token_b = login_as(&ctx, email_b, "student").await;

    // A lanza su job
    let resp_a = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token_a}"))
        .await;

    resp_a.assert_status(StatusCode::ACCEPTED);

    // B también puede lanzar el suyo simultáneamente
    let resp_b = ctx
        .server
        .post("/subjects/auto-select")
        .add_header("Authorization", format!("Bearer {token_b}"))
        .await;

    resp_b.assert_status(StatusCode::ACCEPTED);

    // Los job_id deben ser distintos
    let job_id_a = extraer_job_id(&resp_a.json::<serde_json::Value>());
    let job_id_b = extraer_job_id(&resp_b.json::<serde_json::Value>());
    assert_ne!(
        job_id_a, job_id_b,
        "cada usuario debe tener su propio job_id"
    );
}
