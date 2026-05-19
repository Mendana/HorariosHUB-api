use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::classes::{
        models::{
            CreateClassRequest, CreateClassResponse, Session, UpdateClassRequest,
            UpdateClassResponse,
        },
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

/// Modifica una sesión existente
///
/// Marca automáticamente is_overridden a true para que el scrapper sepa que hay
/// cambios manuales aplicados y no sobreescriba sin gestionar el conflicto
///
/// # Errores
/// - [`AppError::BadRequest`] si la fecha o el formato de hora son inválidos
/// - [`AppError::NotFound`] si no se encuentra la sesión a modificar
pub async fn update_class(
    repo: &dyn ClassRepository,
    id: Uuid,
    payload: UpdateClassRequest,
) -> Result<UpdateClassResponse, AppError> {
    let existing = repo.find_by_id(id).await?.ok_or(AppError::NotFound)?;

    // Parsear fecha (si viene)
    let new_date = payload
        .date
        .as_ref()
        .map(|d| {
            NaiveDate::from_ymd_opt(d.year, d.month, d.day)
                .ok_or_else(|| AppError::BadRequest("Fecha inválida".into()))
        })
        .transpose()?;

    // Parsear hora de inicio (si viene)
    let new_start_time = payload
        .start_time
        .as_deref()
        .map(|t| {
            NaiveTime::parse_from_str(t, "%H:%M").map_err(|_| {
                AppError::BadRequest("Formato de hora de inicio inválido, usar HH:MM".into())
            })
        })
        .transpose()?;

    // Calcular starts_at si cambia fecha u hora
    let new_starts_at = match (new_date, new_start_time) {
        (None, None) => None,
        (date, time) => {
            let date = date.unwrap_or_else(|| existing.starts_at.date_naive());
            let time = time.unwrap_or_else(|| existing.starts_at.time());
            Some(Utc.from_utc_datetime(&NaiveDateTime::new(date, time)))
        }
    };

    // Calcular duración efectiva (si viene nueva duración usarla, sino mantener la existente)
    let effective_duration = payload.duration_minutes.unwrap_or(existing.duration_min);

    // Calcular end_time a partir de starts_at efectiva y duración efectiva
    let effective_start = new_starts_at.unwrap_or(existing.starts_at);

    // Calcular end_time a partir de effective_start y effective_duration
    let end_time = effective_start.time() + Duration::minutes(effective_duration as i64);

    let session = repo
        .update_session(
            id,
            payload.name.as_deref(),
            payload.r#type.as_deref(),
            new_starts_at,
            payload.duration_minutes,
            payload.classroom.as_deref(),
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
