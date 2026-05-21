use axum_test::TestServer;
use serde_json::json;
use sqlx::PgPool;
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

pub struct TestContext {
    pub server: TestServer,
    pub pool: PgPool,
    _container: ContainerAsync<Postgres>,
}

pub async fn setup() -> TestContext {
    let container = Postgres::default()
        .start()
        .await
        .expect("No se pudo levantar el contenedor de Postgres");

    let db_url = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        container
            .get_host_port_ipv4(5432)
            .await
            .expect("No se pudo obtener el puerto")
    );

    let (app, pool) = horarioshub_api::create_test_app(&db_url).await;
    let server = TestServer::new(app);

    TestContext {
        server,
        pool,
        _container: container,
    }
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
