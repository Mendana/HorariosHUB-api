use super::models::{User, UserRole};
use crate::errors::AppError;
use sqlx::PgPool;

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
    async fn create(
        &self,
        email: &str,
        password_hash: &str,
        role: UserRole,
    ) -> Result<User, AppError>;
}

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserRepository for PgUserRepository {
    /// Busca un usuario por su email
    ///
    /// Devuelve `None` si no existe
    ///
    /// # Errores:
    /// - [`AppError::Internal`] para errores en la consulta a la base de datos
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as!(
            User,
            r#"
        SELECT id, email, password_hash, role AS "role: UserRole", verified
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Inserta un nuevo usuario en la base de datos y devuelve el usuario creado
    ///
    /// # Errores:
    /// - [`AppError::Internal`] para errores en la inserción en la base de datos (e.g. conexión, constraints)
    async fn create(
        &self,
        email: &str,
        password_hash: &str,
        role: UserRole,
    ) -> Result<User, AppError> {
        let user = sqlx::query_as!(
            User,
            r#"
        INSERT INTO users (email, password_hash, role)
        VALUES ($1, $2, $3)
        RETURNING
        id,
        email,
        password_hash,
        role AS "role: UserRole",
        verified
        "#,
            email,
            password_hash,
            role as UserRole
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }
}
