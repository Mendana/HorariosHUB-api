use crate::common::{login_as, setup};
use axum::http::StatusCode;
use uuid::Uuid;

async fn get_user_id(ctx: &crate::common::TestContext, email: &str) -> String {
    sqlx::query_scalar!("SELECT id FROM users WHERE email = $1", email)
        .fetch_one(&ctx.pool)
        .await
        .expect("usuario no encontrado en la DB")
        .to_string()
}

// ─── Autenticación / autorización ─────────────────────────────────────────────

#[tokio::test]
async fn change_role_devuelve_401_sin_autenticar() {
    let ctx = setup().await;
    let id = Uuid::new_v4();

    let response = ctx
        .server
        .patch(&format!("/users/{id}/change/professor"))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn change_role_devuelve_403_como_student() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student_change@uniovi.es", "student").await;
    let id = Uuid::new_v4();

    let response = ctx
        .server
        .patch(&format!("/users/{id}/change/professor"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn change_role_devuelve_403_como_profesor() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_change@uniovi.es", "professor").await;
    let id = Uuid::new_v4();

    let response = ctx
        .server
        .patch(&format!("/users/{id}/change/student"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

// ─── Casos válidos ────────────────────────────────────────────────────────────

#[tokio::test]
async fn change_role_devuelve_200_y_actualiza_el_role() {
    let ctx = setup().await;
    let admin_token = login_as(&ctx, "admin_change@uniovi.es", "admin").await;
    login_as(&ctx, "target_change@uniovi.es", "student").await;
    let user_id = get_user_id(&ctx, "target_change@uniovi.es").await;

    let response = ctx
        .server
        .patch(&format!("/users/{user_id}/change/professor"))
        .add_header("Authorization", format!("Bearer {admin_token}"))
        .await;

    response.assert_status(StatusCode::OK);

    let role: String = sqlx::query_scalar!(
        "SELECT role::text FROM users WHERE id = $1",
        uuid::Uuid::parse_str(&user_id).unwrap()
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap()
    .unwrap();

    assert_eq!(role, "professor");
}

#[tokio::test]
async fn change_role_permite_promover_a_admin() {
    let ctx = setup().await;
    let admin_token = login_as(&ctx, "admin_promote@uniovi.es", "admin").await;
    login_as(&ctx, "target_promote@uniovi.es", "student").await;
    let user_id = get_user_id(&ctx, "target_promote@uniovi.es").await;

    let response = ctx
        .server
        .patch(&format!("/users/{user_id}/change/admin"))
        .add_header("Authorization", format!("Bearer {admin_token}"))
        .await;

    response.assert_status(StatusCode::OK);
}

// ─── Casos de error en el input ───────────────────────────────────────────────

#[tokio::test]
async fn change_role_devuelve_400_con_role_invalido() {
    let ctx = setup().await;
    let admin_token = login_as(&ctx, "admin_bad_role@uniovi.es", "admin").await;
    let id = Uuid::new_v4();

    let response = ctx
        .server
        .patch(&format!("/users/{id}/change/superadmin"))
        .add_header("Authorization", format!("Bearer {admin_token}"))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn change_role_devuelve_400_con_id_no_uuid() {
    let ctx = setup().await;
    let admin_token = login_as(&ctx, "admin_bad_uuid@uniovi.es", "admin").await;

    let response = ctx
        .server
        .patch("/users/not-a-uuid/change/professor")
        .add_header("Authorization", format!("Bearer {admin_token}"))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn change_role_devuelve_404_si_el_usuario_no_existe() {
    let ctx = setup().await;
    let admin_token = login_as(&ctx, "admin_404@uniovi.es", "admin").await;
    let id = Uuid::new_v4();

    let response = ctx
        .server
        .patch(&format!("/users/{id}/change/professor"))
        .add_header("Authorization", format!("Bearer {admin_token}"))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}
