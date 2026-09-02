use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::{
        notifications::models::{NewNotification, Notification, NotificationType, NotifyRecipient},
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

    /// Devuelve las notificaciones del usuario, más recientes primero, junto con el
    /// total de filas que cumplen el filtro (para paginar).
    async fn get_notifications_by_user_id(
        &self,
        user_id: Uuid,
        read: Option<bool>,
        offset: u32,
        limit: u32,
    ) -> Result<(Vec<Notification>, u32), AppError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Notification>, AppError>;

    async fn mark_as_read(&self, id: Uuid) -> Result<(), AppError>;

    async fn mark_all_as_read(&self, user_id: Uuid) -> Result<u32, AppError>;

    async fn delete_by_id(&self, id: Uuid) -> Result<(), AppError>;
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
    #[tracing::instrument(skip(self), fields(session_id = %session_id))]
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

    #[tracing::instrument(skip(self), fields(roles = ?roles))]
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

    #[tracing::instrument(skip(self, notifications), fields(count = notifications.len()))]
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

    #[tracing::instrument(skip(self), fields(user_id = %user_id))]
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

    #[tracing::instrument(skip(self), fields(user_id = %user_id, read = ?read, offset = %offset, limit = %limit))]
    async fn get_notifications_by_user_id(
        &self,
        user_id: Uuid,
        read: Option<bool>,
        offset: u32,
        limit: u32,
    ) -> Result<(Vec<Notification>, u32), AppError> {
        let notifications = sqlx::query_as!(
            Notification,
            r#"
            SELECT
                n.id,
                n.user_id,
                n.type as "type: NotificationType",
                n.title,
                n.body,
                n.session_id,
                n.proposal_id,
                n.read,
                n.created_at
            FROM notifications n
            WHERE n.user_id = $1
              AND ($2::boolean IS NULL OR n.read = $2)
            ORDER BY n.created_at DESC
            OFFSET $3
            LIMIT $4
            "#,
            user_id,
            read,
            offset as i64,
            limit as i64
        )
        .fetch_all(&self.pool)
        .await?;

        let total = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) FROM notifications n
            WHERE n.user_id = $1
              AND ($2::boolean IS NULL OR n.read = $2)
            "#,
            user_id,
            read
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0) as u32;

        Ok((notifications, total))
    }

    #[tracing::instrument(skip(self), fields(notification_id = %id))]
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Notification>, AppError> {
        let notification = sqlx::query_as!(
            Notification,
            r#"
            SELECT
                id,
                user_id,
                type as "type: NotificationType",
                title,
                body,
                session_id,
                proposal_id,
                read,
                created_at
            FROM notifications
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(notification)
    }

    #[tracing::instrument(skip(self), fields(notification_id = %id))]
    async fn mark_as_read(&self, id: Uuid) -> Result<(), AppError> {
        sqlx::query!("UPDATE notifications SET read = true WHERE id = $1", id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[tracing::instrument(skip(self), fields(user_id = %user_id))]
    async fn mark_all_as_read(&self, user_id: Uuid) -> Result<u32, AppError> {
        let result = sqlx::query!(
            "UPDATE notifications SET read = true WHERE user_id = $1 AND read = false",
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as u32)
    }

    #[tracing::instrument(skip(self), fields(notification_id = %id))]
    async fn delete_by_id(&self, id: Uuid) -> Result<(), AppError> {
        sqlx::query!("DELETE FROM notifications WHERE id = $1", id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
