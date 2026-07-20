use sqlx::PgPool;
use uuid::Uuid;

use crate::{errors::AppError, modules::user_metrics::models::UserSessionRow};

#[async_trait::async_trait]
pub trait UserMetricsRepository: Send + Sync {
    async fn fetch_all_user_rows(&self, user_id: &Uuid) -> Result<Vec<UserSessionRow>, AppError>;
}

pub struct PgUserMetricsRepository {
    pool: PgPool,
}

impl PgUserMetricsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserMetricsRepository for PgUserMetricsRepository {
    async fn fetch_all_user_rows(&self, user_id: &Uuid) -> Result<Vec<UserSessionRow>, AppError> {
        let rows = sqlx::query_as!(
            UserSessionRow,
            r#"
            SELECT
                s.subject,
                s.grp,
                se.starts_at,
                se.duration_min
            FROM schedule s
            JOIN sessions se
            ON se.subject = s.subject AND se.grp = s.grp
            WHERE s.user_id = $1
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}
