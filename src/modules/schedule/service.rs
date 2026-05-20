use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::schedule::{
        models::{ScheduleResponse, ScheduleSubject},
        repository::ScheduleRepository,
    },
};

pub async fn get_user_weekly_schedule(
    repository: &dyn ScheduleRepository,
    identifier: &Uuid,
    start_time: DateTime<Utc>,
) -> Result<ScheduleResponse, AppError> {
    let rows = repository
        .fetch_user_weekly_schedule_rows(identifier, start_time)
        .await?;

    let sessions: Vec<ScheduleSubject> = rows
        .into_iter()
        .map(|row| ScheduleSubject {
            id: row.id,
            subject: row.subject,
            group: row.group,
            start_time: row.starts_at,
            end_time: row.starts_at + Duration::minutes(row.duration_min.into()),
        })
        .collect();

    Ok(ScheduleResponse { sessions })
}
