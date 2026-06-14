use chrono::{DateTime, Duration, Months, NaiveDate, NaiveTime, TimeZone, Utc};

use crate::{
    errors::AppError,
    modules::{
        auth::{models::User, repository::UserRepository},
        schedule::{
            models::{
                CopyScheduleResponse, CopyScheduleUsers, GetScheduleResponse, ScheduleSubject,
                ScheduleSubjectRow,
            },
            repository::ScheduleRepository,
        },
    },
};

pub async fn get_user_weekly_schedule(
    user_repo: &dyn UserRepository,
    schedule_repo: &dyn ScheduleRepository,
    identifier: &str,
    start_time: DateTime<Utc>,
) -> Result<GetScheduleResponse, AppError> {
    let user = resolve_user(user_repo, identifier).await?;
    let end_time = start_time + Duration::days(7);
    let rows = schedule_repo
        .fetch_user_schedule_rows(&user.id, start_time, end_time)
        .await?;
    Ok(GetScheduleResponse {
        sessions: rows_to_sessions(rows),
    })
}

pub async fn get_user_monthly_schedule(
    user_repo: &dyn UserRepository,
    schedule_repo: &dyn ScheduleRepository,
    identifier: &str,
    month: &str,
) -> Result<GetScheduleResponse, AppError> {
    let user = resolve_user(user_repo, identifier).await?;
    let naive_start = NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("El mes debe tener el formato YYYY-MM".to_string()))?;
    let naive_end = naive_start
        .checked_add_months(Months::new(1))
        .ok_or_else(|| AppError::BadRequest("Mes fuera de rango".to_string()))?;
    let start_time = Utc.from_utc_datetime(&naive_start.and_time(NaiveTime::MIN));
    let end_time = Utc.from_utc_datetime(&naive_end.and_time(NaiveTime::MIN));

    let rows = schedule_repo
        .fetch_user_schedule_rows(&user.id, start_time, end_time)
        .await?;
    Ok(GetScheduleResponse {
        sessions: rows_to_sessions(rows),
    })
}

pub async fn copy_schedule(
    user_repo: &dyn UserRepository,
    schedule_repo: &dyn ScheduleRepository,
    from_email: &str,
    to_user: &User,
) -> Result<CopyScheduleResponse, AppError> {
    if from_email == to_user.email {
        return Err(AppError::BadRequest(
            "Cannot copy schedule to self".to_string(),
        ));
    }
    let from_user = resolve_user(user_repo, from_email).await?;
    let copied_rows = schedule_repo
        .copy_schedule_rows(&CopyScheduleUsers {
            from_user: from_user.id,
            to_user: to_user.id,
        })
        .await?;
    Ok(CopyScheduleResponse {
        message: "Copy was successful".into(),
        copied_count: Some(copied_rows.count),
    })
}

async fn resolve_user(user_repo: &dyn UserRepository, identifier: &str) -> Result<User, AppError> {
    if identifier.chars().count() < 6 {
        return Err(AppError::BadRequest(format!(
            "Invalid identifier: {identifier}"
        )));
    }
    user_repo
        .find_by_email(identifier)
        .await?
        .ok_or(AppError::NotFound)
}

fn rows_to_sessions(rows: Vec<ScheduleSubjectRow>) -> Vec<ScheduleSubject> {
    rows.into_iter()
        .map(|row| ScheduleSubject {
            id: row.id,
            subject: row.subject,
            group: row.group,
            start_time: row.starts_at,
            end_time: row.starts_at + Duration::minutes(row.duration_min.into()),
        })
        .collect()
}
