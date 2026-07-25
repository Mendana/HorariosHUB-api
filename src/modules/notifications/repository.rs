use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::{
        notifications::models::{NewNotification, NotificationType, NotifyRecipient},
        users::models::UserRole,
    },
};

#[async_trait::async_trait]
pub trait NotificationRepository: Send + Sync {
    async fn find_subscribers_by_session(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<NotifyRecipient>, AppError>;

    async fn find_users_by_roles(
        &self,
        roles: &[UserRole],
    ) -> Result<Vec<NotifyRecipient>, AppError>;

    async fn insert_many(&self, notifications: &[NewNotification]) -> Result<(), AppError>;

    async fn find_user_by_id(&self, user_id: Uuid) -> Result<Option<NotifyRecipient>, AppError>;
}

pub struct PgNotificationRepository {
    pub pool: PgPool,
}

impl PgNotificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl NotificationRepository for PgNotificationRepository {
    async fn find_subscribers_by_session(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<NotifyRecipient>, AppError> {
        let recipients = sqlx::query_as!(
            NotifyRecipient,
            r#"
            SELECT
                u.id as user_id,
                u.email,
                u.notify_email,
                u.notify_in_app
            FROM sessions se
            JOIN schedule s ON s.subject = se.subject AND s.grp = se.grp
            JOIN users u ON u.id = s.user_id
            WHERE se.id = $1
            "#,
            session_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(recipients)
    }

    async fn find_users_by_roles(
        &self,
        roles: &[UserRole],
    ) -> Result<Vec<NotifyRecipient>, AppError> {
        let recipients = sqlx::query_as!(
            NotifyRecipient,
            r#"
            SELECT
                u.id as user_id,
                u.email,
                u.notify_email,
                u.notify_in_app
            FROM users u
            WHERE u.role = ANY($1::user_role[])
            "#,
            roles as &[UserRole]
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(recipients)
    }

    async fn insert_many(&self, notifications: &[NewNotification]) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        for notification in notifications {
            sqlx::query!(
                r#"
                INSERT INTO notifications (user_id, type, title, body, session_id, proposal_id)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
                notification.user_id,
                notification.r#type.clone() as NotificationType,
                notification.title,
                notification.body,
                notification.session_id,
                notification.proposal_id
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn find_user_by_id(&self, user_id: Uuid) -> Result<Option<NotifyRecipient>, AppError> {
        let recipient = sqlx::query_as!(
            NotifyRecipient,
            r#"
            SELECT
                u.id as user_id,
                u.email,
                u.notify_email,
                u.notify_in_app
            FROM users u
            WHERE u.id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(recipient)
    }
}
