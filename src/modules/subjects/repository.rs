use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::subjects::models::{JobStatusRow, SubjectGroupRow},
};

#[async_trait::async_trait]
pub trait SubjectRepository: Send + Sync {
    async fn get_catalog_by_user(&self, user_id: Uuid) -> Result<Vec<SubjectGroupRow>, AppError>;

    async fn set_user_selection_destructive(
        &self,
        user_id: Uuid,
        groups_ids: Vec<Uuid>,
    ) -> Result<(), AppError>;

    async fn set_user_selection_by_subject_grp(
        &self,
        user_id: Uuid,
        groups: Vec<(String, String)>,
    ) -> Result<i32, AppError>;

    async fn create_auto_select_job(&self, user_id: Uuid) -> Result<Uuid, AppError>;

    async fn get_active_job_for_user(&self, user_id: Uuid) -> Result<Option<Uuid>, AppError>;

    async fn complete_auto_select_job(
        &self,
        job_id: Uuid,
        groups_selected: i32,
    ) -> Result<(), AppError>;

    async fn fail_auto_select_job(&self, job_id: Uuid, error: &str) -> Result<(), AppError>;

    async fn get_latest_job_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<JobStatusRow>, AppError>;
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

    async fn set_user_selection_by_subject_grp(
        &self,
        user_id: Uuid,
        groups: Vec<(String, String)>,
    ) -> Result<i32, AppError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query!(r#"DELETE FROM schedule WHERE user_id = $1"#, user_id)
            .execute(&mut *tx)
            .await?;

        let count = groups.len() as i32;
        for (subject, grp) in &groups {
            sqlx::query!(
                r#"
                INSERT INTO schedule (user_id, subject, grp)
                VALUES ($1, $2, $3)
                ON CONFLICT DO NOTHING
                "#,
                user_id,
                subject,
                grp
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(count)
    }

    async fn create_auto_select_job(&self, user_id: Uuid) -> Result<Uuid, AppError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"
            DELETE FROM auto_select_jobs
            WHERE user_id = $1 AND status != 'processing'::auto_select_job_status
            "#,
            user_id
        )
        .execute(&mut *tx)
        .await?;

        let row = sqlx::query!(
            r#"
            INSERT INTO auto_select_jobs (user_id, status)
            VALUES ($1, 'processing'::auto_select_job_status)
            RETURNING id
            "#,
            user_id
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(row.id)
    }

    async fn get_active_job_for_user(&self, user_id: Uuid) -> Result<Option<Uuid>, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT id
            FROM auto_select_jobs
            WHERE user_id = $1 AND status = 'processing'::auto_select_job_status
            LIMIT 1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.id))
    }

    async fn complete_auto_select_job(
        &self,
        job_id: Uuid,
        groups_selected: i32,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            UPDATE auto_select_jobs
            SET status = 'completed'::auto_select_job_status,
                groups_selected = $2,
                finished_at = NOW()
            WHERE id = $1
            "#,
            job_id,
            groups_selected
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn fail_auto_select_job(&self, job_id: Uuid, error: &str) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            UPDATE auto_select_jobs
            SET status = 'failed'::auto_select_job_status,
                error = $2,
                finished_at = NOW()
            WHERE id = $1
            "#,
            job_id,
            error
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_latest_job_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<JobStatusRow>, AppError> {
        let row = sqlx::query_as!(
            JobStatusRow,
            r#"
            SELECT id, status::text AS "status!", groups_selected, error
            FROM auto_select_jobs
            WHERE user_id = $1
            ORDER BY started_at DESC
            LIMIT 1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }
}
