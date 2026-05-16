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

async fn login_user(server: &TestServer, email: &str, password: &str) -> String {
    let response = server
        .post("/auth/login")
        .json(&json!({
            "email": email,
            "password": password
        }))
        .await;

    response.assert_status_ok();

    // Extraer el token de la cookie Set-Cookie
    let cookie_header = response
        .headers()
        .get("set-cookie")
        .expect("No hay cookie de autenticación")
        .to_str()
        .expect("No se pudo convertir cookie a string");

    // Extraer el valor del access_token
    cookie_header
        .split("access_token=")
        .nth(1)
        .expect("No se encontró access_token")
        .split(';')
        .next()
        .expect("No se pudo extraer el token")
        .to_string()
}

#[tokio::test]
async fn get_auth_me_devuelve_200_con_usuario_autenticado() {
    let (server, _container) = setup().await;

    // Registrar usuario
    let email = "me@uniovi.es";
    let password = "password123";
    server
        .post("/auth/register")
        .json(&json!({
            "email": email,
            "password": password
        }))
        .await;

    // Hacer login para obtener el token
    let token = login_user(&server, email, password).await;

    // Llamar a GET /auth/me con el token
    let response = server
        .get("/auth/me")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();

    // Verificar que la respuesta contiene el email y rol
    let body: serde_json::Value = response.json();
    assert_eq!(body["email"], email);
    assert_eq!(body["role"], "student");
}

#[tokio::test]
async fn get_auth_me_devuelve_401_sin_token() {
    let (server, _container) = setup().await;

    let response = server.get("/auth/me").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_auth_me_devuelve_401_con_token_invalido() {
    let (server, _container) = setup().await;

    let response = server
        .get("/auth/me")
        .add_header("Authorization", "Bearer invalid_token_xyz")
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_auth_me_devuelve_401_con_bearer_invalido() {
    let (server, _container) = setup().await;

    let token = "valid_jwt_token_would_go_here";

    // Sin el prefijo "Bearer "
    let response = server
        .get("/auth/me")
        .add_header("Authorization", token)
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_auth_me_devuelve_200_con_datos_correctos() {
    let (server, _container) = setup().await;

    // Registrar usuario
    let email = "verify@uniovi.es";
    let password = "password123";
    server
        .post("/auth/register")
        .json(&json!({
            "email": email,
            "password": password
        }))
        .await;

    // Hacer login
    let token = login_user(&server, email, password).await;

    // Llamar a GET /auth/me
    let response = server
        .get("/auth/me")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();

    // Verificar estructura completa de la respuesta
    let body: serde_json::Value = response.json();
    assert!(body.is_object());
    assert_eq!(body["email"], email);
    assert_eq!(body["role"], "student");
    assert!(!body["role"].is_null());
    assert!(!body["email"].is_null());
}

#[tokio::test]
async fn get_auth_me_devuelve_token_expiration_error() {
    let (server, _container) = setup().await;

    // Usar un token que ya expiró (simular con un JWT malformado que falla la validación)
    let expired_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwiZW1haWwiOiJ0ZXN0QHVuaW92aS5lcyIsInJvbGUiOiJzdHVkZW50IiwiZXhwIjoxNjAwMDAwMDAwLCJpYXQiOjE2MDAwMDAwMDB9.invalid_signature";

    let response = server
        .get("/auth/me")
        .add_header("Authorization", format!("Bearer {expired_token}"))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_auth_me_con_multiples_usuarios() {
    let (server, _container) = setup().await;

    // Crear dos usuarios
    let email1 = "user1@uniovi.es";
    let email2 = "user2@uniovi.es";
    let password = "password123";

    server
        .post("/auth/register")
        .json(&json!({
            "email": email1,
            "password": password
        }))
        .await;

    server
        .post("/auth/register")
        .json(&json!({
            "email": email2,
            "password": password
        }))
        .await;

    // Obtener tokens para cada uno
    let token1 = login_user(&server, email1, password).await;
    let token2 = login_user(&server, email2, password).await;

    // Verificar que cada token devuelve al usuario correcto
    let response1 = server
        .get("/auth/me")
        .add_header("Authorization", format!("Bearer {token1}"))
        .await;
    response1.assert_status_ok();
    assert_eq!(response1.json::<serde_json::Value>()["email"], email1);

    let response2 = server
        .get("/auth/me")
        .add_header("Authorization", format!("Bearer {token2}"))
        .await;
    response2.assert_status_ok();
    assert_eq!(response2.json::<serde_json::Value>()["email"], email2);
}

#[cfg(test)]
mod jwt_unit_tests {
    use horarioshub_api::jwt;
    use horarioshub_api::modules::auth::models::UserRole;
    use uuid::Uuid;

    #[test]
    fn test_generate_token_creates_valid_jwt() {
        let user_id = Uuid::new_v4().to_string();
        let email = "test@uniovi.es";
        let role = UserRole::Student;
        let secret = "test_secret_key";
        let ttl = 3600u64;

        let token = jwt::generate_token(&user_id, email, &role, secret, ttl)
            .expect("No se pudo generar el token");

        // El token no debe estar vacío
        assert!(!token.is_empty());
        // El token debe tener el formato JWT (3 partes separadas por puntos)
        assert_eq!(token.split('.').count(), 3);
    }

    #[test]
    fn test_verify_token_extracts_claims() {
        let user_id = Uuid::new_v4().to_string();
        let email = "test@uniovi.es";
        let role = UserRole::Student;
        let secret = "test_secret_key";
        let ttl = 3600u64;

        let token = jwt::generate_token(&user_id, email, &role, secret, ttl)
            .expect("No se pudo generar el token");

        let claims = jwt::verify_token(&token, secret).expect("No se pudo verificar el token");

        // Verificar que los claims se extrajeron correctamente
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.email, email);
        assert_eq!(claims.role, role);
    }

    #[test]
    fn test_verify_token_fails_with_wrong_secret() {
        let user_id = Uuid::new_v4().to_string();
        let email = "test@uniovi.es";
        let role = UserRole::Student;
        let secret = "test_secret_key";
        let ttl = 3600u64;

        let token = jwt::generate_token(&user_id, email, &role, secret, ttl)
            .expect("No se pudo generar el token");

        // Intentar verificar con un secret diferente
        let result = jwt::verify_token(&token, "different_secret");

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token_fails_with_invalid_token() {
        let secret = "test_secret_key";
        let invalid_token = "not.a.valid.jwt.token";

        let result = jwt::verify_token(invalid_token, secret);

        assert!(result.is_err());
    }

    #[test]
    fn test_claims_include_all_required_fields() {
        let user_id = Uuid::new_v4().to_string();
        let email = "test@uniovi.es";
        let role = UserRole::Student;
        let secret = "test_secret_key";
        let ttl = 3600u64;

        let token = jwt::generate_token(&user_id, email, &role, secret, ttl)
            .expect("No se pudo generar el token");

        let claims = jwt::verify_token(&token, secret).expect("No se pudo verificar el token");

        // Verificar que todos los campos requeridos estén presentes
        assert!(!claims.sub.is_empty());
        assert!(!claims.email.is_empty());
        assert!(claims.exp > 0);
        assert!(claims.iat > 0);
        assert!(claims.exp > claims.iat); // exp debe ser mayor que iat
    }

    #[test]
    fn test_token_expiration_fields_are_valid() {
        let user_id = Uuid::new_v4().to_string();
        let email = "test@uniovi.es";
        let role = UserRole::Student;
        let secret = "test_secret_key";
        let ttl = 3600u64;

        let token = jwt::generate_token(&user_id, email, &role, secret, ttl)
            .expect("No se pudo generar el token");

        let claims = jwt::verify_token(&token, secret).expect("No se pudo verificar el token");

        // El token debe expirar en aproximadamente ttl segundos
        let expiration_diff = (claims.exp - claims.iat) as u64;
        assert!(expiration_diff >= ttl - 10 && expiration_diff <= ttl + 10);
    }
}
