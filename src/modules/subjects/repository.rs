use sqlx::PgPool;
use uuid::Uuid;

use crate::{errors::AppError, modules::subjects::models::SubjectGroupRow};

#[async_trait::async_trait]
pub trait SubjectRepository: Send + Sync {
    async fn get_catalog_by_user(&self, user_id: Uuid) -> Result<Vec<SubjectGroupRow>, AppError>;

    async fn set_user_selection_destructive(
        &self,
        user_id: Uuid,
        groups_ids: Vec<Uuid>,
    ) -> Result<(), AppError>;
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

    async fn set_user_selection_destructive(
        &self,
        user_id: Uuid,
        groups_ids: Vec<Uuid>,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        // Delete existing selections for the user
        sqlx::query!(
            r#"
            DELETE FROM schedule
            WHERE user_id = $1
            "#,
            user_id
        )
        .execute(&mut *tx)
        .await?;

        // Insert new selections
        for group_id in groups_ids {
            sqlx::query!(
                r#"
                INSERT INTO schedule (user_id, subject, grp)
                SELECT $1, subject, grp
                FROM subject_groups
                WHERE id = $2
                "#,
                user_id,
                group_id
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }
}
