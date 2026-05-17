use crate::common::{login_user, setup};
use serde_json::json;

#[tokio::test]
async fn post_logout_con_inicio_sesion_devuelve_200() {
    let ctx = setup().await;

    let email = "me@uniovi.es";
    let password = "password123";
    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": email, "password": password }))
        .await;

    let token = login_user(&ctx.server, email, password).await;

    let response = ctx
        .server
        .post("/auth/logout")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status_ok();
}

#[tokio::test]
async fn post_logout_borra_cookie_access_token() {
    let ctx = setup().await;
    let email = "logout_cookie@uniovi.es";
    let password = "password123";

    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": email, "password": password }))
        .await;

    let token = login_user(&ctx.server, email, password).await;

    let response = ctx
        .server
        .post("/auth/logout")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    let set_cookie = response
        .headers()
        .get("set-cookie")
        .expect("Debe haber cabecera set-cookie")
        .to_str()
        .unwrap();

    assert!(
        set_cookie.contains("access_token=;"),
        "La cookie debe quedar vacía"
    );
    assert!(
        set_cookie.contains("HttpOnly"),
        "La cookie debe ser HttpOnly"
    );
    assert!(
        set_cookie.contains("SameSite=Strict"),
        "La cookie debe ser SameSite=Strict"
    );
}

#[tokio::test]
async fn post_logout_sin_autenticacion_devuelve_401() {
    let ctx = setup().await;

    let response = ctx.server.post("/auth/logout").await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn post_logout_con_token_invalido_devuelve_401() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/auth/logout")
        .add_header("Authorization", "Bearer token_inventado_invalido")
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn post_logout_con_token_malformado_devuelve_401() {
    let ctx = setup().await;

    let response = ctx
        .server
        .post("/auth/logout")
        .add_header(
            "Authorization",
            "Bearer eyJhbGciOiJIUzI1NiJ9.malformado.xxx",
        )
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}
