use sqlx::PgPool;
use uuid::Uuid;

use crate::{errors::AppError, modules::subjects::models::SubjectGroupRow};

#[async_trait::async_trait]
pub trait SubjectRepository: Send + Sync {
    async fn get_catalog_by_user(&self, user_id: Uuid) -> Result<Vec<SubjectGroupRow>, AppError>;
}

pub struct PgSubjectRepository {
    pool: PgPool,
}

impl PgSubjectRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl SubjectRepository for PgSubjectRepository {
    async fn get_catalog_by_user(&self, user_id: Uuid) -> Result<Vec<SubjectGroupRow>, AppError> {
        let rows = sqlx::query_as!(
            SubjectGroupRow,
            r#"
            SELECT sg.id, sg.subject, sg.grp, (s.user_id IS NOT NULL) AS "selected!"
            FROM subject_groups sg
            LEFT JOIN schedule s ON sg.subject = s.subject AND s.grp = sg.grp AND s.user_id = $1
            ORDER BY sg.subject, sg.grp
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}
