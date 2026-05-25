use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::{
        classes::{
            models::{
                CreateClassRequest, CreateClassResponse, DeleteClassResponse, Session,
                UpdateClassRequest, UpdateClassResponse,
            },
            repository::ClassRepository,
        },
        proposals::{
            models::{ChangeStatus, ChangeType, CreateChangeInput},
            repository::ProposalRepository,
        },
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
    class_repo: &dyn ClassRepository,
    proposals_repo: &dyn ProposalRepository,
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

    class_repo
        .upsert_subject_group(&payload.name, &payload.r#type)
        .await?;

    let session = class_repo
        .create_session(
            &payload.name,
            &payload.r#type,
            starts_at,
            payload.duration_minutes,
            payload.classroom.as_deref(),
            created_by,
        )
        .await?;

    proposals_repo
        .create_change(CreateChangeInput {
            proposed_by: created_by,
            change_type: ChangeType::Create,
            session_id: None,
            subject: Some(session.subject.clone()),
            grp: Some(session.grp.clone()),
            new_starts_at: Some(session.starts_at),
            new_duration: Some(session.duration_min),
            new_classroom: session.classroom.clone(),
            prev_starts_at: None,
            prev_duration: None,
            prev_classroom: None,
            status: ChangeStatus::Approved,
        })
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
    class_repo: &dyn ClassRepository,
    proposals_repo: &dyn ProposalRepository,
    id: Uuid,
    professor_id: Uuid,
    payload: UpdateClassRequest,
) -> Result<UpdateClassResponse, AppError> {
    let existing = class_repo.find_by_id(id).await?.ok_or(AppError::NotFound)?;

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

    let session = class_repo
        .update_session(
            id,
            payload.name.as_deref(),
            payload.r#type.as_deref(),
            new_starts_at,
            payload.duration_minutes,
            payload.classroom.as_deref(),
        )
        .await?;

    proposals_repo
        .create_change(CreateChangeInput {
            proposed_by: professor_id,
            change_type: ChangeType::Modify,
            session_id: Some(session.id),
            subject: None,
            grp: None,
            new_starts_at: session
                .starts_at
                .ne(&existing.starts_at)
                .then_some(session.starts_at),
            new_duration: session
                .duration_min
                .ne(&existing.duration_min)
                .then_some(session.duration_min),
            new_classroom: session.classroom.clone(),
            prev_starts_at: Some(existing.starts_at),
            prev_duration: Some(existing.duration_min),
            prev_classroom: existing.classroom.clone(),
            status: ChangeStatus::Approved,
        })
        .await?;

    Ok(build_response(session, end_time))
}

/// Elimina una sesión existente por su ID
///
/// # Errores
/// - [`AppError::NotFound`] si no se encuentra la sesión a eliminar
/// - [`AppError::AppError`] si ocurre un error durante la operación de eliminación
pub async fn delete_class(
    class_repo: &dyn ClassRepository,
    proposals_repo: &dyn ProposalRepository,
    id: Uuid,
    professor_id: Uuid,
) -> Result<DeleteClassResponse, AppError> {
    // Verificar que la sesión existe antes de intentar eliminarla
    let session = class_repo.find_by_id(id).await?.ok_or(AppError::NotFound)?;

    proposals_repo
        .create_change(CreateChangeInput {
            proposed_by: professor_id,
            change_type: ChangeType::Delete,
            session_id: Some(id),
            subject: None,
            grp: None,
            new_starts_at: None,
            new_duration: None,
            new_classroom: None,
            prev_starts_at: Some(session.starts_at),
            prev_duration: Some(session.duration_min),
            prev_classroom: session.classroom.clone(),
            status: ChangeStatus::Approved,
        })
        .await?;

    class_repo.delete_session(id).await?;

    Ok(DeleteClassResponse {
        message: "Clase eliminada".into(),
    })
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
