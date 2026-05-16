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

#[tokio::test]
async fn post_login_devuelve_200_y_cookie() {
    let (server, _container) = setup().await;

    // Primero registrar
    server
        .post("/auth/register")
        .json(&json!({ "email": "login@uniovi.es", "password": "password123" }))
        .await;

    // Luego login
    let response = server
        .post("/auth/login")
        .json(&json!({ "email": "login@uniovi.es", "password": "password123" }))
        .await;

    response.assert_status_ok();

    // Verificar body
    let body: serde_json::Value = response.json();
    assert_eq!(body["user"]["email"], "login@uniovi.es");
    assert_eq!(body["user"]["role"], "student");

    // Verificar que la cookie existe
    let cookie_header = response.headers().get("set-cookie");
    assert!(cookie_header.is_some());
    let cookie = cookie_header.unwrap().to_str().unwrap();
    assert!(cookie.contains("access_token="));
    assert!(cookie.contains("HttpOnly"));
}

#[tokio::test]
async fn post_login_devuelve_401_si_password_incorrecta() {
    let (server, _container) = setup().await;

    server
        .post("/auth/register")
        .json(&json!({ "email": "login2@uniovi.es", "password": "password123" }))
        .await;

    let response = server
        .post("/auth/login")
        .json(&json!({ "email": "login2@uniovi.es", "password": "wrongpassword" }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn post_login_devuelve_401_si_email_no_existe() {
    let (server, _container) = setup().await;

    let response = server
        .post("/auth/login")
        .json(&json!({ "email": "noexiste@uniovi.es", "password": "password123" }))
        .await;

    // Mismo error que contraseña incorrecta — no filtramos qué emails existen
    response.assert_status(StatusCode::UNAUTHORIZED);
}
