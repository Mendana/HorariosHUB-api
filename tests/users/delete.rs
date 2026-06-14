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
async fn delete_user_devuelve_401_sin_autenticar() {
    let ctx = setup().await;
    let id = Uuid::new_v4();

    let response = ctx.server.delete(&format!("/users/{id}")).await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn delete_user_devuelve_403_como_student() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student_delete@uniovi.es", "student").await;
    let id = Uuid::new_v4();

    let response = ctx
        .server
        .delete(&format!("/users/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn delete_user_devuelve_403_como_profesor() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_delete@uniovi.es", "professor").await;
    let id = Uuid::new_v4();

    let response = ctx
        .server
        .delete(&format!("/users/{id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

// ─── Casos válidos ────────────────────────────────────────────────────────────

#[tokio::test]
async fn delete_user_devuelve_204_y_elimina_el_usuario() {
    let ctx = setup().await;
    let admin_token = login_as(&ctx, "admin_del@uniovi.es", "admin").await;
    login_as(&ctx, "target_del@uniovi.es", "student").await;
    let user_id = get_user_id(&ctx, "target_del@uniovi.es").await;

    let response = ctx
        .server
        .delete(&format!("/users/{user_id}"))
        .add_header("Authorization", format!("Bearer {admin_token}"))
        .await;

    response.assert_status(StatusCode::NO_CONTENT);

    let exists: Option<Uuid> = sqlx::query_scalar!(
        "SELECT id FROM users WHERE email = $1",
        "target_del@uniovi.es"
    )
    .fetch_optional(&ctx.pool)
    .await
    .unwrap();

    assert!(
        exists.is_none(),
        "el usuario debe haberse eliminado de la DB"
    );
}

// ─── Casos de error en el input ───────────────────────────────────────────────

#[tokio::test]
async fn delete_user_devuelve_400_con_id_no_uuid() {
    let ctx = setup().await;
    let admin_token = login_as(&ctx, "admin_del_bad_uuid@uniovi.es", "admin").await;

    let response = ctx
        .server
        .delete("/users/not-a-uuid")
        .add_header("Authorization", format!("Bearer {admin_token}"))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn delete_user_devuelve_404_si_el_usuario_no_existe() {
    let ctx = setup().await;
    let admin_token = login_as(&ctx, "admin_del_404@uniovi.es", "admin").await;
    let id = Uuid::new_v4();

    let response = ctx
        .server
        .delete(&format!("/users/{id}"))
        .add_header("Authorization", format!("Bearer {admin_token}"))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}
