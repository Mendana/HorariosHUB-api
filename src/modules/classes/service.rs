use chrono::{Duration, NaiveDate, NaiveTime, TimeZone, Utc, Weekday};
use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::{
        classes::{
            models::{
                ClassItem, CreateClassRequest, DeleteClassResponse, ListClassesQueryParams,
                ListClassesResponse, ListClassesSortDirection, ListClassesSortOption,
                ListSessionsParams, Session, UpdateClassRequest,
            },
            repository::ClassRepository,
        },
        notifications::{
            models::SessionChangeType, repository::NotificationRepository,
            service as notifications_service, service::EmailQueue,
        },
        proposals::{
            models::{ChangeStatus, ChangeType, CreateChangeInput},
            repository::ProposalRepository,
        },
    },
};

pub async fn create_class(
    class_repo: &dyn ClassRepository,
    proposals_repo: &dyn ProposalRepository,
    notifications_repo: &dyn NotificationRepository,
    email_queue: &EmailQueue,
    payload: CreateClassRequest,
    created_by: Uuid,
) -> Result<ClassItem, AppError> {
    tracing::debug!(subject = ?payload.subject, group_id = ?payload.group_id, "Creando clase");

    let duration_min = (payload.end_time - payload.start_time).num_minutes();
    validate_duration(duration_min)?;

    let duration_min = duration_min as i32;

    let (subject, subject_type) = resolve_subject_group(
        payload.group_id,
        payload.subject,
        payload.subject_type,
        class_repo,
    )
    .await?;

    class_repo
        .upsert_subject_group(&subject, &subject_type)
        .await
        .map_err(|e| {
            tracing::error!(?e, "Error al upsertar subject y subject_type");
            e
        })?;

    let session = class_repo
        .create_session(
            &subject,
            &subject_type,
            payload.start_time,
            duration_min,
            payload.classroom.as_deref(),
            created_by,
        )
        .await
        .map_err(|e| {
            tracing::error!(?e, "Error al crear sesión");
            e
        })?;

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
        .await
        .map_err(|e| {
            tracing::error!(?e, "Error al crear propuesta de cambio");
            e
        })?;

    tracing::info!(session_id = %session.id, subject = %session.subject, grp = %session.grp, created_by = %created_by, "Clase creada");

    if session.grp.eq_ignore_ascii_case("examen") {
        notifications_service::notify_exam_added(
            notifications_repo,
            email_queue,
            session.id,
            &session.subject,
            &session.grp,
            session.starts_at,
        )
        .await;
    }

    Ok(session_to_class_item(session))
}

pub async fn update_class(
    class_repo: &dyn ClassRepository,
    proposals_repo: &dyn ProposalRepository,
    notifications_repo: &dyn NotificationRepository,
    email_queue: &EmailQueue,
    id: Uuid,
    professor_id: Uuid,
    payload: UpdateClassRequest,
) -> Result<ClassItem, AppError> {
    tracing::debug!(session_id = %id, "Actualizando clase");

    let existing = class_repo.find_by_id(id).await?.ok_or_else(|| {
        tracing::warn!(session_id = %id, "Clase no encontrada");
        AppError::NotFound
    })?;

    // Si viene group_id, resuelve subject/subject_type a partir de él
    let (resolved_subject, resolved_subject_type) = if payload.group_id.is_some() {
        let (s, st) = resolve_subject_group(payload.group_id, None, None, class_repo)
            .await
            .map_err(|e| {
                tracing::error!(?e, "Error al resolver subject y subject_type");
                e
            })?;
        (Some(s), Some(st))
    } else {
        (payload.subject, payload.subject_type)
    };

    let new_starts_at = payload.start_time;
    let new_duration_min = match (payload.start_time, payload.end_time) {
        (Some(start), Some(end)) => {
            let mins = (end - start).num_minutes();
            validate_duration(mins)?;
            Some(mins as i32)
        }
        (Some(_start), None) => None, // mantener duración original
        (None, Some(end)) => {
            let mins = (end - existing.starts_at).num_minutes();
            validate_duration(mins)?;
            Some(mins as i32)
        }
        (None, None) => None,
    };

    let session = class_repo
        .update_session(
            id,
            resolved_subject.as_deref(),
            resolved_subject_type.as_deref(),
            new_starts_at,
            new_duration_min,
            payload.classroom.as_deref(),
        )
        .await
        .map_err(|e| {
            tracing::error!(?e, "Error al actualizar sesión");
            e
        })?;

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
        .await
        .map_err(|e| {
            tracing::error!(?e, "Error al crear propuesta de cambio");
            e
        })?;

    tracing::info!(session_id = %session.id, subject = %session.subject, grp = %session.grp, updated_by = %professor_id, "Clase actualizada");

    notifications_service::notify_session_modified(
        notifications_repo,
        email_queue,
        session.id,
        &session.subject,
        &session.grp,
        SessionChangeType::Modified,
    )
    .await;

    Ok(session_to_class_item(session))
}

/// Elimina una sesión existente por su ID
///
/// # Errores
/// - [`AppError::NotFound`] si no se encuentra la sesión a eliminar
/// - [`AppError::AppError`] si ocurre un error durante la operación de eliminación
pub async fn delete_class(
    class_repo: &dyn ClassRepository,
    proposals_repo: &dyn ProposalRepository,
    notifications_repo: &dyn NotificationRepository,
    email_queue: &EmailQueue,
    id: Uuid,
    professor_id: Uuid,
) -> Result<DeleteClassResponse, AppError> {
    tracing::debug!(session_id = %id, "Eliminando clase");

    let session = class_repo.find_by_id(id).await?.ok_or_else(|| {
        tracing::warn!(session_id = %id, "Clase no encontrada para eliminar");
        AppError::NotFound
    })?;

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

    notifications_service::notify_session_modified(
        notifications_repo,
        email_queue,
        id,
        &session.subject,
        &session.grp,
        SessionChangeType::Deleted,
    )
    .await;

    class_repo.delete_session(id).await?;

    tracing::info!(
        session_id = %id,
        subject = %session.subject,
        grp = %session.grp,
        deleted_by = %professor_id,
        "Clase eliminada"
    );
    Ok(DeleteClassResponse {
        message: "Clase eliminada".into(),
    })
}

pub async fn get_classes(
    class_repo: &dyn ClassRepository,
    params: ListClassesQueryParams,
) -> Result<ListClassesResponse, AppError> {
    let (week_start, week_end) = match &params.week {
        Some(w) => {
            let (start, end) = parse_iso_week(w)?;
            (Some(start), Some(end))
        }
        None => (None, None),
    };

    let page = params.page.unwrap_or(1).max(1) as i64;
    let limit = params.limit.unwrap_or(20).min(100) as i64;
    let offset = (page - 1) * limit;

    let order_col = match params.sort.unwrap_or(ListClassesSortOption::Date) {
        ListClassesSortOption::Name => "subject",
        ListClassesSortOption::Type => "grp",
        ListClassesSortOption::Date => "starts_at",
    };
    let order_dir = match params.dir.unwrap_or(ListClassesSortDirection::Asc) {
        ListClassesSortDirection::Asc => "ASC",
        ListClassesSortDirection::Desc => "DESC",
    };

    let (rows, total) = class_repo
        .list_sessions(ListSessionsParams {
            search: params.search.as_deref(),
            week_start,
            week_end,
            order_col,
            order_dir,
            limit,
            offset,
        })
        .await?;

    let classes = rows
        .into_iter()
        .map(|r| ClassItem {
            id: r.id,
            subject: r.subject,
            subject_type: r.grp,
            classroom: r.classroom,
            start_time: r.starts_at,
            end_time: r.starts_at + Duration::minutes(r.duration_min as i64),
        })
        .collect();

    Ok(ListClassesResponse { classes, total })
}

async fn resolve_subject_group(
    group_id: Option<Uuid>,
    subject: Option<String>,
    subject_type: Option<String>,
    class_repo: &dyn ClassRepository,
) -> Result<(String, String), AppError> {
    if let Some(gid) = group_id {
        return class_repo
            .find_subject_group_by_id(gid)
            .await?
            .ok_or(AppError::NotFound);
    }
    match (subject, subject_type) {
        (Some(s), Some(st)) => Ok((s, st)),
        _ => Err(AppError::BadRequest(
            "Se requiere groupId o los campos subject y subjectType".into(),
        )),
    }
}

fn parse_iso_week(week: &str) -> Result<(chrono::DateTime<Utc>, chrono::DateTime<Utc>), AppError> {
    let err =
        || AppError::BadRequest("El formato de semana debe ser YYYY-Www (ej: 2026-W24)".into());

    let (year_str, week_str) = week.split_once("-W").ok_or_else(err)?;
    let year: i32 = year_str.parse().map_err(|_| err())?;
    let week_num: u32 = week_str.parse().map_err(|_| err())?;

    let monday = NaiveDate::from_isoywd_opt(year, week_num, Weekday::Mon).ok_or_else(err)?;
    let next_monday = monday + Duration::days(7);

    Ok((
        Utc.from_utc_datetime(&monday.and_time(NaiveTime::MIN)),
        Utc.from_utc_datetime(&next_monday.and_time(NaiveTime::MIN)),
    ))
}

fn validate_duration(minutes: i64) -> Result<(), AppError> {
    if minutes <= 0 {
        return Err(AppError::BadRequest(
            "endTime debe ser posterior a startTime".into(),
        ));
    }
    if minutes % 30 != 0 {
        return Err(AppError::BadRequest(
            "La duración debe ser múltiplo de 30 minutos".into(),
        ));
    }
    Ok(())
}

fn session_to_class_item(session: Session) -> ClassItem {
    let end_time = session.starts_at + Duration::minutes(session.duration_min as i64);
    ClassItem {
        id: session.id,
        subject: session.subject,
        subject_type: session.grp,
        classroom: session.classroom,
        start_time: session.starts_at,
        end_time,
    }
}
