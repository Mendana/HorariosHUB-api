use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::Type)]
#[sqlx(type_name = "notification_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    SessionModified,
    SessionDeleted,
    ExamAdded,
    ProposalApproved,
    ProposalRejected,
    ProposalCreated,
    ScrapperConflict,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub r#type: NotificationType,
    pub title: String,
    pub body: String,
    pub session_id: Option<Uuid>,
    pub proposal_id: Option<Uuid>,
    pub read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewNotification {
    pub user_id: Uuid,
    pub r#type: NotificationType,
    pub title: String,
    pub body: String,
    pub session_id: Option<Uuid>,
    pub proposal_id: Option<Uuid>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NotifyRecipient {
    pub user_id: Uuid,
    pub email: String,
    pub notify_in_app: bool,
    pub notify_email: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum SessionChangeType {
    Modified,
    Deleted,
}

#[derive(Debug, Clone)]
pub struct ScraperConflictInfo {
    pub subject: String,
    pub grp: String,
    pub prev_starts_at: DateTime<Utc>,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetNotificationsQuery {
    pub read: Option<bool>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

/// DTO de respuesta de `GET /notifications`: igual que `Notification` pero sin `user_id`
/// (ya implícito por ser el usuario autenticado) y con las claves en camelCase.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationItem {
    pub id: Uuid,
    pub r#type: NotificationType,
    pub title: String,
    pub body: String,
    pub session_id: Option<Uuid>,
    pub proposal_id: Option<Uuid>,
    pub read: bool,
    pub created_at: DateTime<Utc>,
}

impl From<Notification> for NotificationItem {
    fn from(n: Notification) -> Self {
        Self {
            id: n.id,
            r#type: n.r#type,
            title: n.title,
            body: n.body,
            session_id: n.session_id,
            proposal_id: n.proposal_id,
            read: n.read,
            created_at: n.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub page: u32,
    pub limit: u32,
    pub total: u32,
    pub total_pages: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GetNotificationsResponse {
    pub data: Vec<NotificationItem>,
    pub pagination: Pagination,
}

/// Respuesta de `PATCH /notifications/{id}/read`
#[derive(Debug, Clone, Serialize)]
pub struct MarkNotificationReadResponse {
    pub id: Uuid,
    pub read: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnreadNotificationsCountResponse {
    pub unread_count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkAllNotificationsReadResponse {
    pub updated: u32,
}
