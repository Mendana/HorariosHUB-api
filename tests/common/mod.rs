use axum_test::TestServer;
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
