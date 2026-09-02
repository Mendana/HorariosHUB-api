use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::schedule::models::{CopiedRows, CopyScheduleUsers, ScheduleSubjectRow},
};

#[async_trait::async_trait]
pub trait ScheduleRepository: Send + Sync {
    /// Busca las sesiones programadas para un usuario en un rango de fechas
    ///
    /// # Errores:
    /// - [`AppError::Internal`] para errores en la consulta a la base de datos (e.g. conexión)
    async fn fetch_user_schedule_rows(
        &self,
        user_id: &Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<ScheduleSubjectRow>, AppError>;

    /// Copiar horario de otro usuario al propio
    ///
    /// # Errores
    /// - [`AppError::Internal`] para errores en la consulta a la base de datos (e.g. conexión)
    async fn copy_schedule_rows(&self, request: &CopyScheduleUsers)
    -> Result<CopiedRows, AppError>;
}

pub struct PgScheduleRepository {
    pool: PgPool,
}

impl PgScheduleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ScheduleRepository for PgScheduleRepository {
    async fn fetch_user_schedule_rows(
        &self,
        user_id: &Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<ScheduleSubjectRow>, AppError> {
        let rows = sqlx::query_as!(
            ScheduleSubjectRow,
            r#"
        SELECT
            se.id,
            s.subject,
            s.grp AS "group",
            se.starts_at,
            se.duration_min,
            se.classroom
        FROM schedule s
        JOIN sessions se
        ON se.subject = s.subject
        AND se.grp = s.grp
        WHERE s.user_id = $1
        AND se.starts_at >= $2
        AND se.starts_at < $3
        "#,
            user_id,
            start_time,
            end_time
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn copy_schedule_rows(&self, users: &CopyScheduleUsers) -> Result<CopiedRows, AppError> {
        let count = sqlx::query!(
            r#"
        INSERT INTO schedule (user_id, subject, grp)
        SELECT $1, subject, grp
        FROM schedule
        WHERE user_id = $2
        ON CONFLICT (user_id, subject, grp) DO NOTHING
        "#,
            users.to_user,
            users.from_user
        )
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(CopiedRows { count })
    }
}
