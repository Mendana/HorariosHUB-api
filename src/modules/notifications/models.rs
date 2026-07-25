use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "notification_type", rename_all = "snake_case")]
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
