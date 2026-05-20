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
pub struct ScheduleResponse {
    pub sessions: Vec<ScheduleSubject>,
}

#[derive(Debug, Deserialize)]
pub struct ScheduleQuery {
    pub start: DateTime<Utc>,
}
