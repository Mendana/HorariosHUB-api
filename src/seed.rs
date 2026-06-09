/// Seed de desarrollo: inserta 3 usuarios de prueba si la tabla está vacía.
///
/// | Email               | Contraseña  | Rol       |
/// |---------------------|-------------|-----------|
/// | alumno@uniovi.es    | Password1   | student   |
/// | admin@uniovi.es     | Password1   | admin     |
/// | profesor@uniovi.es  | Password1   | professor |
pub async fn run(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;

    if count > 0 {
        return Ok(());
    }

    // Hasheamos en runtime para garantizar compatibilidad con la crate bcrypt
    let password = "Password1";
    let hash =
        tokio::task::spawn_blocking(move || bcrypt::hash(password, bcrypt::DEFAULT_COST)).await??;

    sqlx::query(
        r#"
        INSERT INTO users (id, email, role, verified, password_hash) VALUES
          ('23fa8ced-1fd2-4c06-bb3a-3a0338f32b4d', 'alumno@uniovi.es',   'student',   true, $1),
          ('2423d570-5b88-4919-a3b6-036e93db57a6', 'admin@uniovi.es',    'admin',     true, $1),
          ('76f543ee-1191-4f4a-91eb-f571a3654de1', 'profesor@uniovi.es', 'professor', true, $1)
        "#,
    )
    .bind(&hash)
    .execute(pool)
    .await?;

    tracing::info!("Seed de desarrollo aplicado (3 usuarios, contraseña: Password1)");
    Ok(())
}
