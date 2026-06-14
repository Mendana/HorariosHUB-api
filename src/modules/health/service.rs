use crate::modules::health::{models::HealthResponse, repository::HealthRepository};

pub async fn check_health(repo: &dyn HealthRepository) -> HealthResponse {
    match repo.ping().await {
        Ok(_) => HealthResponse {
            status: "ok".to_string(),
            db: "ok".to_string(),
        },
        Err(_) => HealthResponse {
            status: "degraded".to_string(),
            db: "error".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::AppError;
    use async_trait::async_trait;

    struct MockHealthRepository {
        healthy: bool,
    }

    #[async_trait]
    impl HealthRepository for MockHealthRepository {
        async fn ping(&self) -> Result<(), AppError> {
            if self.healthy {
                Ok(())
            } else {
                Err(AppError::Internal(anyhow::anyhow!("db down")))
            }
        }
    }

    #[tokio::test]
    async fn check_health_devuelve_ok_cuando_la_bd_responde() {
        let repo = MockHealthRepository { healthy: true };
        let response = check_health(&repo).await;
        assert_eq!(response.status, "ok");
        assert_eq!(response.db, "ok");
    }

    #[tokio::test]
    async fn check_health_devuelve_degraded_cuando_la_bd_falla() {
        let repo = MockHealthRepository { healthy: false };
        let response = check_health(&repo).await;
        assert_eq!(response.status, "degraded");
        assert_eq!(response.db, "error");
    }
}
