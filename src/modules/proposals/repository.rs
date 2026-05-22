use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::proposals::models::{Change, ChangeStatus, ChangeType, CreateChangeInput},
};

#[async_trait::async_trait]
pub trait ProposalRepository: Send + Sync {
    /// Crea una nueva propuesta de cambio.
    ///
    /// # Arguments
    /// * `proposed_by` - El ID del usuario que propone el cambio.
    /// * `change_type` - El tipo de cambio (create, modify, delete).
    /// * `session_id` - El ID de la sesión afectada (para modify y delete).
    /// * `subject` - La asignatura (para create).
    /// * `grp` - El grupo (para create).
    /// * `new_starts_at` - La nueva fecha de inicio (opcional).
    /// * `new_duration` - La nueva duración en minutos (opcional).
    ///
    /// # Returns
    /// * `Result<Change, AppError>` - Devuelve el cambio creado o un `AppError`.
    async fn create_change(&self, input: CreateChangeInput) -> Result<Change, AppError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Change, AppError>;

    async fn approve(&self, id: Uuid) -> Result<(), AppError>;

    async fn reject(&self, id: Uuid) -> Result<(), AppError>;
}

pub struct PgProposalRepository {
    pool: PgPool,
}

impl PgProposalRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ProposalRepository for PgProposalRepository {
    async fn create_change(&self, input: CreateChangeInput) -> Result<Change, AppError> {
        let change = sqlx::query_as!(
            Change,
            r#"
            INSERT INTO changes (
                proposed_by,
                change_type,
                session_id,
                subject,
                grp,
                new_starts_at,
                new_duration,
                new_classroom,
                prev_starts_at,
                prev_duration,
                prev_classroom
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING
                id,
                proposed_by,
                session_id,
                subject,
                grp,
                change_type AS "change_type: ChangeType",
                change_status AS "change_status: ChangeStatus",
                prev_starts_at,
                prev_duration,
                prev_classroom,
                new_starts_at,
                new_duration,
                new_classroom,
                proposed_at
            "#,
            input.proposed_by,
            input.change_type as _,
            input.session_id,
            input.subject,
            input.grp,
            input.new_starts_at,
            input.new_duration,
            input.new_classroom,
            input.prev_starts_at,
            input.prev_duration,
            input.prev_classroom,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(change)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Change, AppError> {
        let change = sqlx::query_as!(
            Change,
            r#"
            SELECT
                id,
                proposed_by,
                session_id,
                subject,
                grp,
                change_type AS "change_type: ChangeType",
                change_status AS "change_status: ChangeStatus",
                prev_starts_at,
                prev_duration,
                prev_classroom,
                new_starts_at,
                new_duration,
                new_classroom,
                proposed_at
            FROM changes
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::NotFound)?;

        Ok(change)
    }

    async fn approve(&self, id: Uuid) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            UPDATE changes
            SET change_status = $1
            WHERE id = $2
            "#,
            ChangeStatus::Approved as ChangeStatus,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn reject(&self, id: Uuid) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            UPDATE changes
            SET change_status = $1
            WHERE id = $2
            "#,
            ChangeStatus::Rejected as ChangeStatus,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
