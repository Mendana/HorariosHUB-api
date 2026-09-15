use crate::common::{login_as, setup};
use axum::http::StatusCode;
use axum_test::multipart::{MultipartForm, Part};
use serde_json::Value;

fn csv_part(content: &str) -> Part {
    Part::bytes(content.as_bytes().to_vec())
        .file_name("usuarios.csv")
        .mime_type("text/csv")
}

// ─── Autenticación / autorización ─────────────────────────────────────────────

#[tokio::test]
async fn import_devuelve_401_sin_autenticar() {
    let ctx = setup().await;
    let form = MultipartForm::new().add_part("file", csv_part("email,password\n"));

    let response = ctx.server.post("/users/import").multipart(form).await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn import_devuelve_403_como_student() {
    let ctx = setup().await;
    let token = login_as(&ctx, "student_import@uniovi.es", "student").await;
    let form = MultipartForm::new().add_part("file", csv_part("email,password\n"));

    let response = ctx
        .server
        .post("/users/import")
        .add_header("Authorization", format!("Bearer {token}"))
        .multipart(form)
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn import_devuelve_403_como_profesor() {
    let ctx = setup().await;
    let token = login_as(&ctx, "prof_import@uniovi.es", "professor").await;
    let form = MultipartForm::new().add_part("file", csv_part("email,password\n"));

    let response = ctx
        .server
        .post("/users/import")
        .add_header("Authorization", format!("Bearer {token}"))
        .multipart(form)
        .await;

    response.assert_status(StatusCode::FORBIDDEN);
}

// ─── Casos válidos ────────────────────────────────────────────────────────────

#[tokio::test]
async fn import_crea_usuarios_nuevos_y_omite_existentes() {
    let ctx = setup().await;
    let admin_token = login_as(&ctx, "admin_import@uniovi.es", "admin").await;
    login_as(&ctx, "infprimero_import@uniovi.es", "student").await;

    let csv = "email,password\n\
               infprimero_import@uniovi.es,Password123\n\
               matprimero_import@uniovi.es,Password123\n";
    let form = MultipartForm::new().add_part("file", csv_part(csv));

    let response = ctx
        .server
        .post("/users/import")
        .add_header("Authorization", format!("Bearer {admin_token}"))
        .multipart(form)
        .await;

    response.assert_status(StatusCode::OK);

    let body: Value = response.json();
    assert_eq!(body["total"], 2);
    assert_eq!(body["created"], 1);
    assert_eq!(body["skipped"], 1);
    assert_eq!(body["failed"], 0);

    let row: (String, bool) = sqlx::query_as("SELECT role::text, verified FROM users WHERE email = $1")
        .bind("matprimero_import@uniovi.es")
        .fetch_one(&ctx.pool)
        .await
        .unwrap();

    assert_eq!(row.0, "student");
    assert!(row.1, "el usuario importado debe quedar verificado");
}

// ─── Casos de error en el input ───────────────────────────────────────────────

#[tokio::test]
async fn import_devuelve_400_si_falta_el_archivo() {
    let ctx = setup().await;
    let admin_token = login_as(&ctx, "admin_import_missing@uniovi.es", "admin").await;
    let form = MultipartForm::new().add_text("not_file", "irrelevante");

    let response = ctx
        .server
        .post("/users/import")
        .add_header("Authorization", format!("Bearer {admin_token}"))
        .multipart(form)
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn import_devuelve_400_si_el_archivo_esta_vacio() {
    let ctx = setup().await;
    let admin_token = login_as(&ctx, "admin_import_empty@uniovi.es", "admin").await;
    let form = MultipartForm::new().add_part("file", csv_part(""));

    let response = ctx
        .server
        .post("/users/import")
        .add_header("Authorization", format!("Bearer {admin_token}"))
        .multipart(form)
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn import_reporta_fila_con_contrasena_debil_sin_fallar_la_peticion() {
    let ctx = setup().await;
    let admin_token = login_as(&ctx, "admin_import_weak@uniovi.es", "admin").await;
    let csv = "email,password\nfisprimero_import@uniovi.es,weak\n";
    let form = MultipartForm::new().add_part("file", csv_part(csv));

    let response = ctx
        .server
        .post("/users/import")
        .add_header("Authorization", format!("Bearer {admin_token}"))
        .multipart(form)
        .await;

    response.assert_status(StatusCode::OK);

    let body: Value = response.json();
    assert_eq!(body["failed"], 1);
    assert_eq!(body["created"], 0);
    assert_eq!(body["details"][0]["status"], "error");
}
