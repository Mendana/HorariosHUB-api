use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::classes::{
        models::{CreateClassRequest, CreateClassResponse, Session},
        repository::ClassRepository,
    },
};

/// Crea una nueva clase (sesión) en el sistema.
///
/// # Flujo
/// 1. Parsea fecha y hora de inicio
/// 2. Calcula el end_time a partir de durationMinutes
/// 3. Upsert del subjectGroup si no existe
/// 4. Inserta la sesón con source manual
///
/// # Errores
/// - [`AppError::BadRequest`] si la fecha o el formato de hora son inválidos
pub async fn create_class(
    repo: &dyn ClassRepository,
    payload: CreateClassRequest,
    created_by: Uuid,
) -> Result<CreateClassResponse, AppError> {
    let date = NaiveDate::from_ymd_opt(payload.date.year, payload.date.month, payload.date.day)
        .ok_or_else(|| AppError::BadRequest("Fecha inválida".into()))?;

    let start_time = NaiveTime::parse_from_str(&payload.start_time, "%H:%M").map_err(|_| {
        AppError::BadRequest("Formato de hora de inicio inválido, usar HH:MM".into())
    })?;

    let end_time = start_time + Duration::minutes(payload.duration_minutes as i64);

    let starts_at = Utc.from_utc_datetime(&NaiveDateTime::new(date, start_time));

    repo.upsert_subject_group(&payload.name, &payload.r#type)
        .await?;

    let session = repo
        .create_session(
            &payload.name,
            &payload.r#type,
            starts_at,
            payload.duration_minutes,
            payload.classroom.as_deref(),
            created_by,
        )
        .await?;

    Ok(build_response(session, end_time))
}

fn build_response(session: Session, end_time: NaiveTime) -> CreateClassResponse {
    let start = session.starts_at.format("%H:%M").to_string();
    let date = session.starts_at.format("%Y-%m-%d").to_string();

    CreateClassResponse {
        id: session.id,
        name: session.subject,
        r#type: session.grp,
        date,
        start_time: start,
        end_time: end_time.format("%H:%M").to_string(),
        duration_minutes: session.duration_min,
        classroom: session.classroom,
    }
}
