use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::classes::models::{ClassItemRow, ListSessionsParams, Session, SessionSource},
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

    /// Busca el ID de una sesión por su clave natural (subject, grp, starts_at).
    ///
    /// Útil cuando `session_id` ha quedado NULL por el ciclo borrar+reinsertar
    /// del scraper (ON DELETE SET NULL) y hay que relocalizar la sesión.
    ///
    /// # Returns
    /// * `Ok(Some(id))` si existe una sesión con esa clave natural.
    /// * `Ok(None)` si no existe.
    async fn find_by_natural_key(
        &self,
        subject: &str,
        grp: &str,
        starts_at: DateTime<Utc>,
    ) -> Result<Option<Uuid>, AppError>;

    /// Devuelve una página de sesiones con filtros opcionales y el total de resultados sin paginar
    ///
    /// # Errores
    /// - [`AppError::Internal`] para errores en la consulta a la base de datos
    async fn list_sessions(
        &self,
        params: ListSessionsParams<'_>,
    ) -> Result<(Vec<ClassItemRow>, i64), AppError>;

    /// Busca el subject y grp de un subject_group por su UUID.
    async fn find_subject_group_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<(String, String)>, AppError>;
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
    #[tracing::instrument(skip(self), fields(subject = %subject, grp = %grp))]
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

    #[tracing::instrument(skip(self, classroom), fields(subject = %subject, grp = %grp, created_by = %created_by))]
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

    #[tracing::instrument(skip(self), fields(session_id = %id))]
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

    #[tracing::instrument(skip(self, subject, grp, starts_at, duration_min, classroom), fields(session_id = %id))]
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

    #[tracing::instrument(skip(self), fields(session_id = %id))]
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

    #[tracing::instrument(skip(self, starts_at), fields(subject = %subject, grp = %grp))]
    async fn find_by_natural_key(
        &self,
        subject: &str,
        grp: &str,
        starts_at: DateTime<Utc>,
    ) -> Result<Option<Uuid>, AppError> {
        let id = sqlx::query_scalar!(
            r#"
            SELECT id FROM sessions
            WHERE subject = $1 AND grp = $2 AND starts_at = $3
            "#,
            subject,
            grp,
            starts_at,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(id)
    }

    #[tracing::instrument(skip(self), fields(group_id = %id))]
    async fn find_subject_group_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<(String, String)>, AppError> {
        let row = sqlx::query!("SELECT subject, grp FROM subject_groups WHERE id = $1", id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| (r.subject, r.grp)))
    }

    #[tracing::instrument(skip(self, params))]
    async fn list_sessions(
        &self,
        params: ListSessionsParams<'_>,
    ) -> Result<(Vec<ClassItemRow>, i64), AppError> {
        let sql = format!(
            r#"
            SELECT
                id,
                subject,
                grp,
                classroom,
                starts_at,
                duration_min,
                COUNT(*) OVER() AS total
            FROM sessions
            WHERE
                ($1::text IS NULL OR subject ILIKE '%' || $1 || '%')
                AND ($2::timestamptz IS NULL OR starts_at >= $2)
                AND ($3::timestamptz IS NULL OR starts_at < $3)
            ORDER BY {order_col} {order_dir}
            LIMIT $4 OFFSET $5
            "#,
            order_col = params.order_col,
            order_dir = params.order_dir,
        );

        let rows = sqlx::query_as::<_, ClassItemRow>(&sql)
            .bind(params.search)
            .bind(params.week_start)
            .bind(params.week_end)
            .bind(params.limit)
            .bind(params.offset)
            .fetch_all(&self.pool)
            .await?;

        let total = rows.first().map(|r| r.total).unwrap_or(0);
        Ok((rows, total))
    }
}
