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
    /// * `Result<(), AppError>` - Devuelve `Ok(())` si la operación fue exitosa, o un `AppError` si ocurrió un error durante la operación.
    ///
    /// # Errors
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
}
