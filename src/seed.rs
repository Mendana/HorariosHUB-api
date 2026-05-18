pub async fn run(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;

    if count > 0 {
        return Ok(());
    }

    sqlx::query(
        r#"
        INSERT INTO users (id, email, role, verified, password_hash) VALUES
          ('23fa8ced-1fd2-4c06-bb3a-3a0338f32b4d', 'alumno@uniovi.es',   'student',   true, '$2b$12$okiHTXQU7IDPdpES5y9rIe1DwGVDm4n6H3hU13ORFRvvrjUbYOoVO'),
          ('2423d570-5b88-4919-a3b6-036e93db57a6', 'admin@uniovi.es',    'admin',     true, '$2b$12$ZJ52vYCBj3YFjaaGBZ6g6uO0eFc2s4mVyrxrIjkz3lvdXJ3xmfPwO'),
          ('76f543ee-1191-4f4a-91eb-f571a3654de1', 'profesor@uniovi.es', 'professor', true, '$2b$12$79DnqZDHeMAkquhjZE.v0uNDs3G9YlTxy1obw2vWJsscmSPlO..QW')
        "#,
    )
    .execute(pool)
    .await?;

    tracing::info!("Seed de desarrollo aplicado (3 usuarios)");
    Ok(())
}
