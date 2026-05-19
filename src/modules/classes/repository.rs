use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::classes::models::{Session, SessionSource},
};

#[async_trait::async_trait]
pub trait ClassRepository: Send + Sync {
    /// Crea o actualiza un grupo de asignaturas. Si el grupo ya existe, se actualiza su información; si no, se crea uno nuevo.
    ///
    /// # Arguments
    /// * `subject` - El nombre de la asignatura.
    /// * `grp` - El nombre del grupo del la asignatura.
    ///
    /// # Returns
    /// * `Result<(), AppError>` - Devuelve `Ok(())` si la operación fue exitosa, o un `AppError` si ocurre un error durante la operación.
    async fn upsert_subject_group(&self, subject: &str, grp: &str) -> Result<(), AppError>;

    /// Crea una nueva sesión de clase con la información proporcionada.
    ///
    /// # Arguments
    /// * `subject` - El nombre de la asignatura.
    /// * `grp` - El nombre del grupo de la asignatura.
    /// * `starts_at` - La fecha y hora de inicio de la sesión.
    /// * `duration_min` - La duración de la sesión en minutos.
    /// * `classroom` - El aula donde se llevará a cabo la sesión (op
    /// * `created_by` - El ID del usuario que creó la sesión.
    async fn create_session(
        &self,
        subject: &str,
        grp: &str,
        starts_at: DateTime<Utc>,
        duration_min: i32,
        classroom: Option<&str>,
        created_by: Uuid,
    ) -> Result<Session, AppError>;

    /// Busca una sesión de clase por su ID.
    ///
    /// # Arguments
    /// * `id` - El ID de la sesión que se desea buscar.
    ///
    /// # Returns
    /// * `Result<Option<Session>, AppError>` - Devuelve `Ok(Some(Session))` si se encuentra la sesión, `Ok(None)` si no se encuentra, o un `AppError` si ocurre un error durante la operación.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Session>, AppError>;

    /// Actualiza los campos de una sesión de clase existente.
    ///
    /// # Arguments
    /// * `id` - El ID de la sesión que se desea actualizar.
    /// * `subject` - El nuevo nombre de la asignatura (opcional).
    /// * `grp` - El nuevo nombre del grupo de la asignatura (opcional)
    /// * `starts_at` - La nueva fecha y hora de inicio de la sesión (opcional).
    /// * `duration_min` - La nueva duración de la sesión en minutos (opcional).
    /// * `classroom` - El nuevo aula donde se llevará a cabo la sesión (opcional).
    ///
    /// # Returns
    /// * `Result<Session, AppError>` - Devuelve la sesión actualizada si la operación fue exitosa, o un `AppError` si ocurre un error durante la operación.
    async fn update_session(
        &self,
        id: Uuid,
        subject: Option<&str>,
        grp: Option<&str>,
        starts_at: Option<DateTime<Utc>>,
        duration_min: Option<i32>,
        classroom: Option<&str>,
    ) -> Result<Session, AppError>;

    /// Elimina una sesión de clase por su ID.
    ///
    /// # Arguments
    /// * `id` - El ID de la sesión que se desea eliminar.
    ///
    /// # Returns
    /// * `Result<(), AppError>` - Devuelve `Ok(())` si la operación fue exitosa, o un `AppError` si ocurre un error durante la operación.
    async fn delete_session(&self, id: Uuid) -> Result<(), AppError>;
}

pub struct PgClassRepository {
    pool: PgPool,
}

impl PgClassRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ClassRepository for PgClassRepository {
    async fn upsert_subject_group(&self, subject: &str, grp: &str) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO subject_groups (subject, grp)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
            subject,
            grp,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn create_session(
        &self,
        subject: &str,
        grp: &str,
        starts_at: DateTime<Utc>,
        duration_min: i32,
        classroom: Option<&str>,
        created_by: Uuid,
    ) -> Result<Session, AppError> {
        let session = sqlx::query_as!(
            Session,
            r#"
            INSERT INTO sessions (
                subject,
                grp,
                starts_at,
                duration_min,
                classroom,
                source,
                created_by,
                is_overridden,
                scraped_at
            )
            VALUES ($1, $2, $3, $4, $5, 'manual', $6, false, NULL)
            RETURNING
                id,
                subject,
                grp,
                starts_at,
                duration_min,
                classroom,
                source AS "source: SessionSource",
                created_by,
                is_overridden
            "#,
            subject,
            grp,
            starts_at,
            duration_min,
            classroom,
            created_by,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(session)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Session>, AppError> {
        let session = sqlx::query_as!(
            Session,
            r#"
            SELECT
                id,
                subject,
                grp,
                starts_at,
                duration_min,
                classroom,
                source AS "source: SessionSource",
                created_by,
                is_overridden
            FROM sessions
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }

    async fn update_session(
        &self,
        id: Uuid,
        subject: Option<&str>,
        grp: Option<&str>,
        starts_at: Option<DateTime<Utc>>,
        duration_min: Option<i32>,
        classroom: Option<&str>,
    ) -> Result<Session, AppError> {
        let session = sqlx::query_as!(
            Session,
            r#"
            UPDATE sessions SET
                subject = COALESCE($2, subject),
                grp = COALESCE($3, grp),
                starts_at = COALESCE($4, starts_at),
                duration_min = COALESCE($5, duration_min),
                classroom = COALESCE($6, classroom),
                is_overridden = true
            WHERE id = $1
            RETURNING
                id,
                subject,
                grp,
                starts_at,
                duration_min,
                classroom,
                source AS "source: SessionSource",
                created_by,
                is_overridden
            "#,
            id,
            subject,
            grp,
            starts_at,
            duration_min,
            classroom,
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::NotFound)?;

        Ok(session)
    }

    async fn delete_session(&self, id: Uuid) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            DELETE FROM sessions
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
