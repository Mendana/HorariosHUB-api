use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::scraper::models::{ApprovedChange, ParsedSession},
};

#[async_trait::async_trait]
pub trait ScraperRepository: Send + Sync {
    /// Intenta adquirir el lock de ejecución.
    ///
    /// Si el lock lleva más de 2 horas activo se considera caducado (proceso caído)
    /// y se roba automáticamente.
    ///
    /// # Args
    /// - `locked_by`: Identificador de quién intenta adquirir el lock (ej: "instance-1")
    ///
    /// # Returns
    /// - `Ok(true)`: Lock adquirido exitosamente (o robado por TTL caducado)
    /// - `Ok(false)`: Lock ya adquirido por otro proceso activo, no se debe proceder
    /// - `Err(e)`: Error de base de datos
    async fn acquire_lock(&self, locked_by: &str) -> Result<bool, AppError>;

    /// Libera el lock de ejecución, permitiendo que otros procesos puedan adquirirlo
    ///
    /// # Returns
    /// - `Ok(())`: Lock liberado exitosamente
    /// - `Err(e)`: Error al intentar liberar el lock (consulta a base de datos fallida, etc)
    async fn release_lock(&self) -> Result<(), AppError>;

    /// Elimina todas las sesiones de la base de datos
    ///
    /// # Returns
    /// - `Ok(count)`: Número de sesiones eliminadas
    async fn delete_all_sessions(&self) -> Result<usize, AppError>;

    /// Inserta un batch de sesiones parseadas en la base de datos
    ///
    /// # Args
    /// - `sessions`: Lista de sesiones parseadas a insertar
    ///
    /// # Returns
    /// - `Ok(count)`: Número de sesiones insertadas exitosamente
    /// - `Err(e)`: Error al intentar insertar las sesiones (consulta a base de datos fallida, etc)
    async fn insert_sessions_batch(&self, sessions: &[ParsedSession]) -> Result<usize, AppError>;

    /// Upsert (insertar o actualizar) un batch de sesiones parseadas en la base de datos, solo para las columnas de subject y grp
    ///
    /// # Args
    /// - `sessions`: Lista de sesiones parseadas a upsertar
    ///
    /// # Returns
    /// - `Ok(())`: Upsert realizado exitosamente
    /// - `Err(e)`: Error al intentar realizar el upsert (consulta a base de datos fallida, etc)
    async fn upsert_subject_groups_batch(&self, sessions: &[ParsedSession])
    -> Result<(), AppError>;

    /// Obtiene la lista de cambios aprobados que aún no han sido aplicados a la tabla de sesiones
    ///
    /// # Returns
    /// - `Ok(changes)`: Lista de cambios aprobados pendientes de aplicación
    /// - `Err(e)`: Error al intentar obtener los cambios aprobados (consulta a base de datos fallida, etc)
    async fn get_approved_changes(&self) -> Result<Vec<ApprovedChange>, AppError>;

    /// Busca el ID de la sesión que coincide con la clave natural (subject, grp, starts_at)
    ///
    /// # Args
    /// - `subject`: Código de la asignatura (ej: "AL")
    /// - `grp`: Código del grupo (ej: "T.1")
    /// - `starts_at`: Fecha y hora de inicio de la sesión
    ///
    /// # Returns
    /// - `Ok(Some(session_id))`: Se encontró una sesión que coincide con la clave natural, y se devuelve su ID
    /// - `Ok(None)`: No se encontró ninguna sesión que coincida con la clave
    /// - `Err(e)`: Error al intentar buscar la sesión (consulta a base de datos fallida, etc)
    async fn find_session_by_natural_key(
        &self,
        subject: &str,
        grp: &str,
        starts_at: DateTime<Utc>,
    ) -> Result<Option<Uuid>, AppError>;

    /// Aplica un cambio aprobado de tipo "modify" a la sesión correspondiente
    ///
    /// # Args
    /// - `session_id`: ID de la sesión a modificar
    /// - `new_starts_at`: Nueva fecha y hora de inicio (opcional, si el cambio no incluye modificación de esta columna, se pasa None)
    /// - `new_duration `: Nueva duración en minutos (opcional, si el cambio no incluye modificación de esta columna, se pasa None)
    /// - `new_classroom`: Nuevo aula (opcional, si el cambio no incluye modificación de esta columna, se pasa None)
    ///
    /// # Returns
    /// - `Ok(())`: Cambio aplicado exitosamente
    /// - `Err(e)`: Error al intentar aplicar el cambio (consulta a base de datos fallida, etc)
    async fn apply_modify(
        &self,
        session_id: Uuid,
        new_starts_at: Option<DateTime<Utc>>,
        new_duration: Option<i32>,
        new_classroom: Option<&str>,
    ) -> Result<(), AppError>;

    /// Elimina una sesión por su ID, aplicando un cambio aprobado de tipo "delete"
    ///
    /// # Args
    /// - `session_id`: ID de la sesión a eliminar
    ///
    /// # Returns
    /// - `Ok(())`: Sesión eliminada exitosamente
    /// - `Err(e)`: Error al intentar eliminar la sesión (consulta a base de datos fallida, etc)
    async fn delete_session_by_id(&self, session_id: Uuid) -> Result<(), AppError>;

    /// Inserta una nueva sesión a partir de un cambio aprobado de tipo "create"
    ///
    /// # Args
    /// - `subject`: Código de la asignatura (ej: "AL")
    /// - `grp`: Código del grupo (ej: "T.1")
    /// - `starts_at`: Fecha y hora de inicio de la sesión
    /// - `duration_min`: Duración de la sesión en minutos
    /// - `classroom`: Aula de la sesión (opcional)
    /// - `created_by`: ID del usuario que propuso el cambio aprobado (para registrar el autor de la sesión creada)
    ///
    /// # Returns
    /// - `Ok(())`: Sesión insertada exitosamente
    /// - `Err(e)`: Error al intentar insertar la sesión (consulta a base de datos fallida, etc)
    async fn insert_session_from_change(
        &self,
        subject: &str,
        grp: &str,
        starts_at: DateTime<Utc>,
        duration_min: i32,
        classroom: Option<&str>,
        created_by: Uuid,
    ) -> Result<(), AppError>;

    /// Rechaza los cambios pendientes que no pudieron ser aplicados por no encontrar una sesión de referencia (huérfanos), y los mueve a histórico
    ///
    /// # Returns
    /// - `Ok(count)`: Número de cambios pendientes rechazados y archivados
    /// - `Err(e)`: Error al intentar rechazar y archivar los cambios pendientes huérfanos (consulta a base de datos fallida, etc)
    async fn archive_rejected_changes(&self) -> Result<usize, AppError>;

    /// Rechaza los cambios pendientes que no pudieron ser aplicados por no encontrar una sesión de referencia (huérfanos), y los mueve a histórico
    ///
    /// # Returns
    /// - `Ok(count)`: Número de cambios pendientes rechazados y archivados
    /// - `Err(e)`: Error al intentar rechazar y archivar los cambios pendientes
    async fn reject_and_archive_orphan_pending(&self) -> Result<usize, AppError>;

    /// Archiva los cambios aprobados de tipo modify/delete cuya sesión de referencia ya no existe
    /// en la base de datos (la fuente oficial cambió la clase desde que se aprobó el cambio).
    ///
    /// Estos cambios se mueven a `changes_history` con su `change_status` original ('approved')
    /// para que quede trazabilidad de que existió la aprobación.
    ///
    /// # Returns
    /// - `Ok(count)`: Número de cambios aprobados huérfanos archivados
    /// - `Err(e)`: Error al intentar archivar los cambios aprobados huérfanos
    async fn archive_orphan_approved_changes(&self) -> Result<usize, AppError>;
}

pub struct PgScraperRepository {
    pool: PgPool,
}

impl PgScraperRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ScraperRepository for PgScraperRepository {
    #[tracing::instrument(skip(self), fields(locked_by = %locked_by))]
    async fn acquire_lock(&self, locked_by: &str) -> Result<bool, AppError> {
        let result = sqlx::query!(
            r#"
            INSERT INTO scraper_locks (id, locked_by, locked_at)
            VALUES ('scraper_run', $1, NOW())
            ON CONFLICT (id) DO UPDATE
                SET locked_by = EXCLUDED.locked_by,
                    locked_at = NOW()
                WHERE scraper_locks.locked_at < NOW() - INTERVAL '2 hours'
            "#,
            locked_by
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    #[tracing::instrument(skip(self))]
    async fn release_lock(&self) -> Result<(), AppError> {
        sqlx::query!("DELETE FROM scraper_locks WHERE id = 'scraper_run'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn delete_all_sessions(&self) -> Result<usize, AppError> {
        let result = sqlx::query!("DELETE FROM sessions")
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() as usize)
    }

    #[tracing::instrument(skip(self, sessions), fields(count = sessions.len()))]
    async fn upsert_subject_groups_batch(
        &self,
        sessions: &[ParsedSession],
    ) -> Result<(), AppError> {
        let mut pairs: Vec<(&str, &str)> = sessions
            .iter()
            .map(|s| (s.subject.as_str(), s.grp.as_str()))
            .collect();
        pairs.sort_unstable();
        pairs.dedup();

        for (subject, grp) in pairs {
            sqlx::query!(
                r#"
                INSERT INTO subject_groups (subject, grp)
                VALUES ($1, $2)
                ON CONFLICT DO NOTHING
                "#,
                subject,
                grp
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, sessions), fields(count = sessions.len()))]
    async fn insert_sessions_batch(&self, sessions: &[ParsedSession]) -> Result<usize, AppError> {
        let mut inserted = 0;

        for session in sessions {
            let result = sqlx::query!(
                r#"
                INSERT INTO sessions (
                    subject,
                    grp,
                    starts_at,
                    duration_min,
                    classroom,
                    source,
                    scraped_at
                )
                VALUES ($1, $2, $3, $4, $5, 'scraper', NOW())
                ON CONFLICT (subject, grp, starts_at) DO NOTHING
                "#,
                session.subject,
                session.grp,
                session.starts_at,
                session.duration_min,
                session.classroom.as_deref()
            )
            .execute(&self.pool)
            .await?;

            inserted += result.rows_affected() as usize;
        }

        Ok(inserted)
    }

    #[tracing::instrument(skip(self))]
    async fn get_approved_changes(&self) -> Result<Vec<ApprovedChange>, AppError> {
        let changes = sqlx::query_as!(
            ApprovedChange,
            r#"
            SELECT
                id,
                change_type AS "change_type: crate::modules::proposals::models::ChangeType",
                subject,
                grp,
                prev_starts_at,
                prev_duration,
                prev_classroom,
                new_starts_at,
                new_duration,
                new_classroom,
                proposed_by,
                proposed_at
            FROM changes
            WHERE change_status = 'approved'
            ORDER BY proposed_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(changes)
    }

    #[tracing::instrument(skip(self, starts_at), fields(subject = %subject, grp = %grp))]
    async fn find_session_by_natural_key(
        &self,
        subject: &str,
        grp: &str,
        starts_at: DateTime<Utc>,
    ) -> Result<Option<Uuid>, AppError> {
        let result = sqlx::query_scalar!(
            r#"
            SELECT id FROM sessions
            WHERE subject = $1 AND grp = $2 AND starts_at = $3
            "#,
            subject,
            grp,
            starts_at
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    #[tracing::instrument(skip(self, new_starts_at, new_duration, new_classroom), fields(session_id = %session_id))]
    async fn apply_modify(
        &self,
        session_id: Uuid,
        new_starts_at: Option<DateTime<Utc>>,
        new_duration: Option<i32>,
        new_classroom: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            UPDATE sessions SET
                starts_at = COALESCE($2, starts_at),
                duration_min = COALESCE($3, duration_min),
                classroom = COALESCE($4, classroom),
                is_overridden = true
            WHERE id = $1
            "#,
            session_id,
            new_starts_at,
            new_duration,
            new_classroom
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[tracing::instrument(skip(self), fields(session_id = %session_id))]
    async fn delete_session_by_id(&self, session_id: Uuid) -> Result<(), AppError> {
        sqlx::query!("DELETE FROM sessions WHERE id=$1", session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[tracing::instrument(
        skip(self, starts_at, duration_min, classroom),
        fields(subject = %subject, grp = %grp, created_by = %created_by)
    )]
    async fn insert_session_from_change(
        &self,
        subject: &str,
        grp: &str,
        starts_at: DateTime<Utc>,
        duration_min: i32,
        classroom: Option<&str>,
        created_by: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO subject_groups (subject, grp)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
            subject,
            grp
        )
        .execute(&self.pool)
        .await?;

        // Si el scrapper ya trajo la sesión, prevalece esa
        sqlx::query!(
            r#"
            INSERT INTO sessions (
                subject, grp, starts_at, duration_min,
                classroom, source, created_by
            )
            VALUES ($1, $2, $3, $4, $5, 'manual', $6)
            ON CONFLICT (subject, grp, starts_at) DO NOTHING
            "#,
            subject,
            grp,
            starts_at,
            duration_min,
            classroom,
            created_by,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn archive_rejected_changes(&self) -> Result<usize, AppError> {
        let result = sqlx::query!(
            r#"
            WITH moved AS (
                DELETE FROM changes
                WHERE change_status = 'rejected'
                RETURNING
                    id, proposed_by, session_id, subject, grp,
                    change_type, change_status, prev_starts_at,
                    prev_duration, new_starts_at, new_duration,
                    proposed_at, prev_classroom, new_classroom
            )
            INSERT INTO changes_history (
                id, proposed_by, session_id, subject, grp,
                change_type, change_status, prev_starts_at,
                prev_duration, new_starts_at, new_duration,
                proposed_at, prev_classroom, new_classroom,
                archived_at
            )
            SELECT
                id, proposed_by, session_id, subject, grp,
                change_type, change_status, prev_starts_at,
                prev_duration, new_starts_at, new_duration,
                proposed_at, prev_classroom, new_classroom,
                NOW()
            FROM moved
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as usize)
    }

    #[tracing::instrument(skip(self))]
    async fn reject_and_archive_orphan_pending(&self) -> Result<usize, AppError> {
        let result = sqlx::query!(
            r#"
            WITH orphans AS (
                DELETE FROM changes
                WHERE change_status = 'pending'
                AND change_type IN ('modify', 'delete')
                AND NOT EXISTS (
                    SELECT 1 FROM sessions s
                    WHERE s.subject = changes.subject
                    AND   s.grp     = changes.grp
                    AND   s.starts_at = changes.prev_starts_at
                )
                RETURNING
                    id, proposed_by, session_id, subject, grp,
                    change_type, change_status, prev_starts_at,
                    prev_duration, new_starts_at, new_duration,
                    proposed_at, prev_classroom, new_classroom
            )
            INSERT INTO changes_history (
                id, proposed_by, session_id, subject, grp,
                change_type, change_status, prev_starts_at,
                prev_duration, new_starts_at, new_duration,
                proposed_at, prev_classroom, new_classroom,
                archived_at
            )
            SELECT
                id, proposed_by, session_id, subject, grp,
                change_type, 'rejected',     -- se archivan como rejected
                prev_starts_at, prev_duration, new_starts_at, new_duration,
                proposed_at, prev_classroom, new_classroom,
                NOW()
            FROM orphans
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as usize)
    }

    #[tracing::instrument(skip(self))]
    async fn archive_orphan_approved_changes(&self) -> Result<usize, AppError> {
        let result = sqlx::query!(
            r#"
            WITH orphans AS (
                DELETE FROM changes
                WHERE change_status = 'approved'
                AND change_type IN ('modify', 'delete')
                AND subject IS NOT NULL
                AND grp IS NOT NULL
                AND prev_starts_at IS NOT NULL
                AND NOT EXISTS (
                    SELECT 1 FROM sessions s
                    WHERE s.subject   = changes.subject
                    AND   s.grp       = changes.grp
                    AND   s.starts_at = changes.prev_starts_at
                )
                RETURNING
                    id, proposed_by, session_id, subject, grp,
                    change_type, change_status, prev_starts_at,
                    prev_duration, new_starts_at, new_duration,
                    proposed_at, prev_classroom, new_classroom
            )
            INSERT INTO changes_history (
                id, proposed_by, session_id, subject, grp,
                change_type, change_status, prev_starts_at,
                prev_duration, new_starts_at, new_duration,
                proposed_at, prev_classroom, new_classroom,
                archived_at
            )
            SELECT
                id, proposed_by, session_id, subject, grp,
                change_type, change_status,  -- se conserva 'approved' para trazabilidad
                prev_starts_at, prev_duration, new_starts_at, new_duration,
                proposed_at, prev_classroom, new_classroom,
                NOW()
            FROM orphans
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as usize)
    }
}
