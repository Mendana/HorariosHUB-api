use crate::common::{login_as, setup};
use axum::http::StatusCode;
use serde_json::json;

// ─── Autenticación / autorización ─────────────────────────────────────────────

#[tokio::test]
async fn get_users_devuelve_401_sin_autenticar() {
    let ctx = setup().await;

    let response = ctx.server.get("/users").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_users_devuelve_403_como_student() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student_users@uniovi.es", "student").await;

    let response = ctx
        .server
        .get("/users")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn get_users_devuelve_200_como_profesor() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_users@uniovi.es", "professor").await;

    let response = ctx
        .server
        .get("/users")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn get_users_devuelve_200_como_admin() {
    let ctx = setup().await;
    let token = login_as(&ctx, "admin_users@uniovi.es", "admin").await;

    let response = ctx
        .server
        .get("/users")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);
}

// ─── Estructura de la respuesta ───────────────────────────────────────────────

#[tokio::test]
async fn get_users_respuesta_tiene_campo_users_array() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_struct_users@uniovi.es", "professor").await;

    let response = ctx
        .server
        .get("/users")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(
        body["users"].is_array(),
        "la respuesta debe tener un campo 'users' de tipo array"
    );
}

#[tokio::test]
async fn get_users_cada_usuario_tiene_email_y_role() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_fields@uniovi.es", "professor").await;

    let response = ctx
        .server
        .get("/users")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    let users = body["users"].as_array().unwrap();
    assert!(
        !users.is_empty(),
        "debe haber al menos un usuario (el propio profesor)"
    );

    for user in users {
        assert!(
            user["email"].is_string(),
            "cada usuario debe tener campo 'email'"
        );
        assert!(
            user["role"].is_string(),
            "cada usuario debe tener campo 'role'"
        );
    }
}

#[tokio::test]
async fn get_users_lista_incluye_usuarios_registrados() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_list_check@uniovi.es", "professor").await;

    // Registrar un alumno adicional
    ctx.server
        .post("/auth/register")
        .json(&json!({ "email": "student_extra@uniovi.es", "password": "Password123" }))
        .await;

    let response = ctx
        .server
        .get("/users")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    let users = body["users"].as_array().unwrap();

    let emails: Vec<&str> = users.iter().map(|u| u["email"].as_str().unwrap()).collect();
    assert!(
        emails.contains(&"prof_list_check@uniovi.es"),
        "el profesor autenticado debe aparecer en la lista"
    );
    assert!(
        emails.contains(&"student_extra@uniovi.es"),
        "el alumno registrado debe aparecer en la lista"
    );
}

#[tokio::test]
async fn get_users_roles_se_serializan_en_minusculas() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_roles@uniovi.es", "professor").await;

    let response = ctx
        .server
        .get("/users")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    let users = body["users"].as_array().unwrap();

    for user in users {
        let role = user["role"].as_str().unwrap();
        assert!(
            matches!(role, "student" | "professor" | "admin"),
            "el role '{role}' no es un valor válido en minúsculas"
        );
    }
}
