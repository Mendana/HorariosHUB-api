use crate::errors::AppError;
use sqlx::PgPool;

#[async_trait::async_trait]
pub trait HealthRepository: Send + Sync {
    async fn ping(&self) -> Result<(), AppError>;
}

pub struct PgHealthRepository {
    pool: PgPool,
}

impl PgHealthRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl HealthRepository for PgHealthRepository {
    async fn ping(&self) -> Result<(), AppError> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }
}
