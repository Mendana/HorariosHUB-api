use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleSubject {
    pub id: Uuid,
    pub subject: String,
    pub group: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ScheduleSubjectRow {
    pub id: Uuid,
    pub subject: String,
    pub group: String,
    pub starts_at: DateTime<Utc>,
    pub duration_min: i32,
}

#[derive(Debug, Serialize, Default)]
pub struct GetScheduleResponse {
    pub sessions: Vec<ScheduleSubject>,
}

#[derive(Debug, Deserialize)]
pub struct GetScheduleQuery {
    pub start: DateTime<Utc>,
}
#[derive(Debug, Deserialize)]
pub struct CopyScheduleQuery {
    pub user: String,
}

pub struct CopyScheduleUsers {
    pub from_user: Uuid,
    pub to_user: Uuid,
}

#[derive(Debug, Serialize, Default)]
pub struct CopyScheduleResponse {
    pub message: String,
    pub copied_count: Option<u64>,
}

pub struct CopiedRows {
    pub count: u64,
}
