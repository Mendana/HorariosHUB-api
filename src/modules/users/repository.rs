use super::models::{PasswordResetToken, User, UserRole, VerificationToken};
use crate::errors::AppError;
use sqlx::PgPool;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    /// Busca un usuario por su ID
    ///
    /// Devuelve `None` si no existe
    ///
    /// # Errores:
    /// - [`AppError::Internal`] para errores en la consulta a la base de datos
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AppError>;

    /// Busca un usuario por su email
    ///
    /// Devuelve `None` si no existe
    ///
    /// # Errores:
    /// - [`AppError::Internal`] para errores en la consulta a la base de datos
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;

    /// Inserta un nuevo usuario en la base de datos y devuelve el usuario creado
    ///
    /// # Errores:
    /// - [`AppError::Internal`] para errores en la inserción en la base de datos (e.g. conexión, constraints)
    async fn create(
        &self,
        email: &str,
        password_hash: &str,
        role: UserRole,
    ) -> Result<User, AppError>;

    /// Crea un token de verificación para el usuario con el ID dado
    /// Devuelve el token generado
    ///
    /// # Errores:
    /// - [`AppError::Internal`] para errores en la inserción en la base de datos (e.g. conexión)
    /// - [`AppError::NotFound`] si no existe un usuario con el ID dado
    /// - [`AppError::Conflict`] si el usuario ya está verificado
    async fn create_verification_token(&self, user_id: uuid::Uuid) -> Result<String, AppError>;

    /// Busca un token de verificación por su valor y devuelve el token encontrado
    /// Devuelve `None` si no existe
    ///
    /// # Errores:
    /// - [`AppError::Internal`] para errores en la consulta a la base de datos (e.g. conexión)
    /// - [`AppError::NotFound`] si no existe un token con el valor dado
    async fn find_verification_token(
        &self,
        token: &str,
    ) -> Result<Option<VerificationToken>, AppError>;

    /// Marca al usuario con el ID dado como verificado
    /// Devuelve `Ok(())` si la operación fue exitosa
    ///
    /// # Errores:
    /// - [`AppError::Internal`] para errores en la actualización en la base de datos
    /// - [`AppError::NotFound`] si no existe un usuario con el ID dado
    /// - [`AppError::Conflict`] si el usuario ya está verificado
    async fn mark_user_as_verified(&self, user_id: uuid::Uuid) -> Result<(), AppError>;

    /// Elimina el token de verificación con el ID dado de la base de datos
    /// Devuelve `Ok(())` si la operación fue exitosa o si el token no existe
    ///
    /// # Errores:
    /// - [`AppError::Internal`] para errores en la eliminación en la base de datos (e.g. conexión)
    async fn delete_verification_token(&self, token_id: uuid::Uuid) -> Result<(), AppError>;

    /// Crea un token de restablecimiento de contraseña para el usuario con el ID dado
    /// Devuelve el token generado
    ///
    /// # Errores:
    /// - [`AppError::Internal`] para errores en la inserción en la base de datos (e.g. conexión)
    /// - [`AppError::NotFound`] si no existe un usuario con el ID dado
    async fn create_password_reset_token(&self, user_id: uuid::Uuid) -> Result<String, AppError>;

    /// Busca un token de restablecimiento de contraseña por su valor y devuelve el token encontrado
    /// Devuelve `None` si no existe
    ///
    /// # Errores:
    /// - [`AppError::Internal`] para errores en la consulta a la base de datos (e.g. conexión)
    /// - [`AppError::NotFound`] si no existe un token con el valor dado
    async fn find_password_reset_token(
        &self,
        token: &str,
    ) -> Result<Option<PasswordResetToken>, AppError>;

    /// Actualiza la contraseña del usuario con el ID dado al nuevo hash de contraseña proporcionado
    /// Devuelve `Ok(())` si la operación fue exitosa
    ///
    /// # Errores:
    /// - [`AppError::Internal`] para errores en la actualización en la base de datos
    /// - [`AppError::NotFound`] si no existe un usuario con el ID dado
    async fn update_password(
        &self,
        user_id: uuid::Uuid,
        new_password_hash: &str,
    ) -> Result<(), AppError>;

    /// Elimina el token de restablecimiento de contraseña con el ID dado de la base de datos
    /// Devuelve `Ok(())` si la operación fue exitosa o si el token no existe
    ///
    /// # Errores:
    /// - [`AppError::Internal`] para errores en la eliminación en la base de datos
    async fn delete_password_reset_token(&self, token_id: uuid::Uuid) -> Result<(), AppError>;

    /// Devuelve todos los usuarios de la base de datos
    ///
    /// # Errores:
    /// - [`AppError::Internal`] para errores en la consulta a la base de datos
    async fn get_all_users(&self) -> Result<Vec<User>, AppError>;

    /// Cambia el rol del usuario con el identificador dado al nuevo rol proporcionado
    /// Devuelve `Ok(())` si la operación fue exitosa
    ///
    /// # Errores:
    /// - [`AppError::Internal`] para errores en la actualización en la base de datos
    /// - [`AppError::NotFound`] si no existe un usuario con el identificador
    async fn change_user_role(&self, identifier: Uuid, new_role: UserRole) -> Result<(), AppError>;

    /// Elimina el usuario con el identificador dado de la base de datos
    ///
    /// # Errores:
    /// - [`AppError::Internal`] para errores en la eliminación en la base de datos
    /// - [`AppError::NotFound`] si no existe un usuario con el identificador dado
    async fn delete_user(&self, identifier: Uuid) -> Result<(), AppError>;
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
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as!(
            User,
            r#"
        SELECT id, email, password_hash, role AS "role: UserRole", verified
            FROM users
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

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

    async fn create_verification_token(&self, user_id: uuid::Uuid) -> Result<String, AppError> {
        // Verificar que el usuario existe y no está verificado
        let user = sqlx::query!("SELECT verified FROM users WHERE id = $1", user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AppError::NotFound)?;

        if user.verified {
            return Err(AppError::Conflict("El usuario ya está verificado".into()));
        }

        // Generar token
        let token = uuid::Uuid::new_v4().to_string();

        sqlx::query!(
            r#"
            INSERT INTO verification_tokens (user_id, token, expires_at)
            VALUES ($1, $2, NOW() + INTERVAL '24 hours')
            "#,
            user_id,
            token
        )
        .execute(&self.pool)
        .await?;

        Ok(token)
    }

    async fn find_verification_token(
        &self,
        token: &str,
    ) -> Result<Option<VerificationToken>, AppError> {
        let record = sqlx::query_as!(
            VerificationToken,
            r#"
            SELECT id, user_id, token, expires_at
            FROM verification_tokens
            WHERE token = $1
            "#,
            token
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    async fn mark_user_as_verified(&self, user_id: uuid::Uuid) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            UPDATE users
            SET verified = true
            WHERE id = $1 AND verified = false
            "#,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_verification_token(&self, token_id: uuid::Uuid) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            DELETE FROM verification_tokens
            WHERE id = $1
            "#,
            token_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn create_password_reset_token(&self, user_id: uuid::Uuid) -> Result<String, AppError> {
        // Invalidar tokens anteriores del mismo usuario
        sqlx::query!(
            "DELETE from password_reset_tokens WHERE user_id = $1",
            user_id
        )
        .execute(&self.pool)
        .await?;

        let token = Uuid::new_v4().to_string();

        sqlx::query!(
            r#"
            INSERT INTO password_reset_tokens (user_id, token, expires_at)
            VALUES ($1, $2, NOW() + INTERVAL '1 hour')
            "#,
            user_id,
            token
        )
        .execute(&self.pool)
        .await?;

        Ok(token)
    }

    async fn find_password_reset_token(
        &self,
        token: &str,
    ) -> Result<Option<PasswordResetToken>, AppError> {
        let record = sqlx::query_as!(
            PasswordResetToken,
            r#"
            SELECT id, user_id, token, expires_at
            FROM password_reset_tokens
            WHERE token = $1
            "#,
            token
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    async fn update_password(
        &self,
        user_id: uuid::Uuid,
        new_password_hash: &str,
    ) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE users SET password_hash = $1 WHERE id = $2",
            new_password_hash,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_password_reset_token(&self, token_id: uuid::Uuid) -> Result<(), AppError> {
        sqlx::query!("DELETE FROM password_reset_tokens WHERE id = $1", token_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get_all_users(&self) -> Result<Vec<User>, AppError> {
        let users = sqlx::query_as!(
            User,
            r#"
            SELECT id, email, password_hash, role AS "role: UserRole", verified
            FROM users
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }

    async fn change_user_role(&self, identifier: Uuid, new_role: UserRole) -> Result<(), AppError> {
        let result = sqlx::query!(
            r#"
            UPDATE users
            SET role = $1
            WHERE id = $2
            "#,
            new_role as UserRole,
            identifier
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }

    async fn delete_user(&self, identifier: Uuid) -> Result<(), AppError> {
        let result = sqlx::query!("DELETE FROM users WHERE id = $1", identifier)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }
}
