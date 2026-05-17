use crate::common::setup;
use axum::http::StatusCode;
use serde_json::json;

async fn setup_reset_token(pool: &sqlx::PgPool, email: &str) -> String {
    // Registrar usuario
    let user_id = sqlx::query_scalar!("SELECT id FROM users WHERE email = $1", email)
        .fetch_optional(pool)
        .await
        .unwrap();

    // Si no existe, insertar directamente para el test
    let user_id = match user_id {
        Some(id) => id,
        None => {
            sqlx::query_scalar!(
                r#"
                INSERT INTO users (email, password_hash, role)
                VALUES ($1, $2, 'student')
                RETURNING id
                "#,
                email,
                bcrypt::hash("password123", 4).unwrap(), // cost 4 — más rápido en tests
            )
            .fetch_one(pool)
            .await
            .unwrap()
        }
    };

    // Crear token de reset directamente en BBDD
    let token = uuid::Uuid::new_v4().to_string();
    sqlx::query!(
        r#"
        INSERT INTO password_reset_tokens (user_id, token, expires_at)
        VALUES ($1, $2, NOW() + INTERVAL '1 hour')
        "#,
        user_id,
        token,
    )
    .execute(pool)
    .await
    .unwrap();

    token
}

#[tokio::test]
async fn post_reset_password_devuelve_200_con_token_valido() {
    let ctx = setup().await;

    let token = setup_reset_token(&ctx.pool, "reset_ok@uniovi.es").await;

    let response = ctx
        .server
        .post("/auth/reset-password")
        .json(&json!({ "token": token, "new_password": "nuevaPassword123" }))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["message"], "Contraseña actualizada");

    // Verificar que el token fue borrado
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM password_reset_tokens WHERE token = $1",
        token
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    assert_eq!(count, Some(0));
}

#[tokio::test]
async fn post_reset_password_devuelve_400_con_token_invalido() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/auth/reset-password")
        .json(&json!({ "token": "token-inventado", "new_password": "nuevaPassword123" }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_reset_password_devuelve_400_con_token_expirado() {
    let ctx = setup().await;

    let token = setup_reset_token(&ctx.pool, "reset_exp@uniovi.es").await;

    // Forzar expiración
    sqlx::query!("UPDATE password_reset_tokens SET expires_at = NOW() - INTERVAL '1 hour'")
        .execute(&ctx.pool)
        .await
        .unwrap();

    let response = ctx
        .server
        .post("/auth/reset-password")
        .json(&json!({ "token": token, "new_password": "nuevaPassword123" }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_reset_password_devuelve_400_si_token_ya_usado() {
    let ctx = setup().await;

    let token = setup_reset_token(&ctx.pool, "reset_reuse@uniovi.es").await;

    let payload = json!({ "token": token, "new_password": "nuevaPassword123" });

    // Primera vez — ok
    ctx.server.post("/auth/reset-password").json(&payload).await;

    // Segunda vez — token borrado
    let response = ctx.server.post("/auth/reset-password").json(&payload).await;
    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_reset_password_devuelve_422_si_password_corta() {
    let ctx = setup().await;

    let token = setup_reset_token(&ctx.pool, "reset_val@uniovi.es").await;

    let response = ctx
        .server
        .post("/auth/reset-password")
        .json(&json!({ "token": token, "new_password": "corta" }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_reset_password_permite_login_con_nueva_contrasena() {
    let ctx = setup().await;

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": "flow@uniovi.es", "password": "oldpassword123" }))
        .await;

    let token = setup_reset_token(&ctx.pool, "flow@uniovi.es").await;

    ctx.server
        .post("/auth/reset-password")
        .json(&json!({ "token": token, "new_password": "newpassword456" }))
        .await
        .assert_status_ok();

    // Login con la nueva contraseña debe funcionar
    let response = ctx
        .server
        .post("/auth/login")
        .json(&json!({ "email": "flow@uniovi.es", "password": "newpassword456" }))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["user"]["email"], "flow@uniovi.es");
}

#[tokio::test]
async fn post_reset_password_invalida_contrasena_anterior() {
    let ctx = setup().await;

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": "oldpwd@uniovi.es", "password": "oldpassword123" }))
        .await;

    let token = setup_reset_token(&ctx.pool, "oldpwd@uniovi.es").await;

    ctx.server
        .post("/auth/reset-password")
        .json(&json!({ "token": token, "new_password": "newpassword456" }))
        .await
        .assert_status_ok();

    // Login con la contraseña antigua debe fallar
    let response = ctx
        .server
        .post("/auth/login")
        .json(&json!({ "email": "oldpwd@uniovi.es", "password": "oldpassword123" }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}
