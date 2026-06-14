use axum_test::TestServer;
use serde_json::json;
use sqlx::PgPool;
use std::sync::{Mutex, OnceLock};
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use tokio::sync::OnceCell;
use uuid::Uuid;

// Un único contenedor para los tests.
static POSTGRES_PORT: OnceCell<u16> = OnceCell::const_new();
static POSTGRES_CONTAINER: OnceLock<Mutex<Option<ContainerAsync<Postgres>>>> = OnceLock::new();

fn container_mutex() -> &'static Mutex<Option<ContainerAsync<Postgres>>> {
    POSTGRES_CONTAINER.get_or_init(|| Mutex::new(None))
}

async fn get_postgres_port() -> u16 {
    *POSTGRES_PORT
        .get_or_init(|| async {
            let container = Postgres::default()
                .start()
                .await
                .expect("No se pudo levantar el contenedor de Postgres");
            let port = container
                .get_host_port_ipv4(5432)
                .await
                .expect("No se pudo obtener el puerto");
            *container_mutex().lock().unwrap() = Some(container);
            port
        })
        .await
}

// Al salir el proceso se limpia el contenedor de Postgres
#[ctor::dtor]
fn cleanup_postgres() {
    let container = match container_mutex().lock().ok().and_then(|mut g| g.take()) {
        Some(c) => c,
        None => return,
    };
    if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        let _ = rt.block_on(container.rm());
    }
}

pub struct TestContext {
    pub server: TestServer,
    pub pool: PgPool,
}

pub async fn setup() -> TestContext {
    setup_with_scraper_url("http://localhost:4000").await
}

pub async fn setup_with_scraper_url(scraper_url: &str) -> TestContext {
    let port = get_postgres_port().await;

    // Base de datos única por test
    let db_name = format!("test_{}", Uuid::new_v4().simple());

    let admin_url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let admin_pool = PgPool::connect(&admin_url)
        .await
        .expect("No se pudo conectar al contenedor de Postgres");
    sqlx::query(&format!("CREATE DATABASE {db_name}"))
        .execute(&admin_pool)
        .await
        .expect("No se pudo crear la base de datos de test");
    admin_pool.close().await;

    let db_url = format!("postgres://postgres:postgres@127.0.0.1:{port}/{db_name}");
    let (app, pool) = horarioshub_api::create_test_app_with_scraper(&db_url, scraper_url).await;
    let server = TestServer::new(app);

    TestContext { server, pool }
}

#[allow(dead_code)]
pub async fn verify_user(pool: &PgPool, email: &str) {
    sqlx::query!("UPDATE users SET verified = true WHERE email = $1", email)
        .execute(pool)
        .await
        .expect("No se pudo verificar el usuario en la DB");
}

#[allow(dead_code)]
pub async fn login_user(server: &TestServer, email: &str, password: &str) -> String {
    use serde_json::json;

    let response = server
        .post("/auth/login")
        .json(&json!({ "email": email, "password": password }))
        .await;

    response.assert_status_ok();

    let cookie_header = response
        .headers()
        .get("set-cookie")
        .expect("No hay cookie de autenticación")
        .to_str()
        .expect("No se pudo convertir cookie a string");

    cookie_header
        .split("access_token=")
        .nth(1)
        .expect("No se encontró access_token")
        .split(';')
        .next()
        .expect("No se pudo extraer el token")
        .to_string()
}

#[allow(dead_code)]
pub async fn login_as(ctx: &crate::common::TestContext, email: &str, role: &str) -> String {
    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": email, "password": "Password123" }))
        .await;

    sqlx::query("UPDATE users SET role = $1::user_role, verified = true WHERE email = $2")
        .bind(role)
        .bind(email)
        .execute(&ctx.pool)
        .await
        .unwrap();

    login_user(&ctx.server, email, "Password123").await
}

#[allow(dead_code)]
pub async fn create_test_session(pool: &sqlx::PgPool) -> uuid::Uuid {
    sqlx::query!(
        "INSERT INTO subject_groups (subject, grp) VALUES ('ALG', 'Teoría') ON CONFLICT DO NOTHING"
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query_scalar!(
        r#"
        INSERT INTO sessions (subject, grp, starts_at, duration_min, source, is_overridden)
        VALUES ('ALG', 'Teoría', '2025-09-15T09:00:00Z', 90, 'scraper', false)
        RETURNING id
        "#
    )
    .fetch_one(pool)
    .await
    .unwrap()
}
