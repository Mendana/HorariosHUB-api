use axum::http::StatusCode;
use axum_test::TestServer;
use serde_json::json;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

async fn setup() -> (TestServer, testcontainers::ContainerAsync<Postgres>) {
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

    let app = horarioshub_api::create_test_app(&db_url).await;
    let server = TestServer::new(app);

    (server, container)
}

#[tokio::test]
async fn post_register_devuelve_201() {
    let (server, _container) = setup().await;

    let response = server
        .post("/auth/register")
        .json(&json!({
            "email": "diego@uniovi.es",
            "password": "password123"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);

    let body: serde_json::Value = response.json();
    assert_eq!(body["email"], "diego@uniovi.es");
    assert_eq!(body["role"], "student");
    assert!(body["id"].is_string());
}

#[tokio::test]
async fn post_register_devuelve_409_si_email_duplicado() {
    let (server, _container) = setup().await;

    let payload = json!({
        "email": "diego@uniovi.es",
        "password": "password123"
    });

    // Primera vez — ok
    server.post("/auth/register").json(&payload).await;

    // Segunda vez con el mismo email — conflicto
    let response = server.post("/auth/register").json(&payload).await;
    response.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn post_register_devuelve_400_si_email_invalido() {
    let (server, _container) = setup().await;

    let response = server
        .post("/auth/register")
        .json(&json!({
            "email": "esto-no-es-un-email",
            "password": "password123"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_register_devuelve_400_si_password_corta() {
    let (server, _container) = setup().await;

    let response = server
        .post("/auth/register")
        .json(&json!({
            "email": "diego@uniovi.es",
            "password": "corta"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}
