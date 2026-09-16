use std::collections::HashMap;

use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::events::{
        models::{
            CreateEventInput, CreateEventRequest, DeleteEventResponse, EventItem, EventOccurrence,
            EventRow, EventRowWithTotal, ListEventsQuery, ListEventsResponse,
            ListOccurrencesQuery, ListOccurrencesResponse, OccurrenceRow, RecurrenceInfo,
            RecurrenceInput, UpdateEventInput, UpdateEventRequest,
        },
        repository::EventRepository,
    },
};

/// Rango máximo permitido en `GET /events/occurrences`, para no generar un
/// número de ocurrencias sin límite (p.ej. un evento diario sin fin cercano).
const MAX_OCCURRENCES_RANGE_DAYS: i64 = 366;

#[tracing::instrument(
    skip(repo, payload),
    fields(subject = %payload.subject, created_by = %created_by)
)]
pub async fn create_event(
    repo: &dyn EventRepository,
    payload: CreateEventRequest,
    created_by: Uuid,
) -> Result<EventItem, AppError> {
    let duration_min = validate_duration(payload.start_time, payload.end_time)?;

    if !repo.subject_exists(&payload.subject).await? {
        return Err(AppError::BadRequest(format!(
            "La asignatura '{}' no existe",
            payload.subject
        )));
    }

    let groups = normalize_groups(payload.groups);
    if !groups.is_empty() {
        validate_groups_exist(repo, &payload.subject, &groups).await?;
    }

    let (recurrence_interval, recurrence_end_date) = match &payload.recurrence {
        Some(r) => {
            validate_recurrence(payload.start_time, r)?;
            (Some(r.interval), Some(r.end_date))
        }
        None => (None, None),
    };

    let row = repo
        .create_event(CreateEventInput {
            title: payload.title,
            description: payload.description,
            subject: payload.subject.clone(),
            starts_at: payload.start_time,
            duration_min,
            classroom: payload.classroom,
            recurrence_interval,
            recurrence_end_date,
            created_by,
        })
        .await?;

    repo.set_event_groups(row.id, &payload.subject, &groups)
        .await?;

    tracing::info!(event_id = %row.id, subject = %row.subject, "Evento creado");

    Ok(event_row_to_item(row, groups))
}

#[tracing::instrument(skip(repo, payload), fields(event_id = %id))]
pub async fn update_event(
    repo: &dyn EventRepository,
    id: Uuid,
    payload: UpdateEventRequest,
) -> Result<EventItem, AppError> {
    let existing = repo.find_by_id(id).await?.ok_or(AppError::NotFound)?;

    let effective_subject = payload.subject.clone().unwrap_or(existing.subject.clone());
    if payload.subject.is_some() && !repo.subject_exists(&effective_subject).await? {
        return Err(AppError::BadRequest(format!(
            "La asignatura '{effective_subject}' no existe"
        )));
    }

    let new_duration = match (payload.start_time, payload.end_time) {
        (Some(start), Some(end)) => Some(validate_duration(start, end)?),
        (Some(start), None) => {
            let end = existing.starts_at + Duration::minutes(existing.duration_min as i64);
            Some(validate_duration(start, end)?)
        }
        (None, Some(end)) => Some(validate_duration(existing.starts_at, end)?),
        (None, None) => None,
    };

    let effective_start = payload.start_time.unwrap_or(existing.starts_at);
    let recurrence = match &payload.recurrence {
        Some(r) => {
            validate_recurrence(effective_start, r)?;
            Some((r.interval, r.end_date))
        }
        None => None,
    };

    let normalized_groups = payload.groups.as_ref().map(|g| normalize_groups_ref(g));
    if let Some(groups) = &normalized_groups
        && !groups.is_empty()
    {
        validate_groups_exist(repo, &effective_subject, groups).await?;
    }

    let row = repo
        .update_event(
            id,
            UpdateEventInput {
                title: payload.title,
                description: payload.description,
                subject: payload.subject,
                starts_at: payload.start_time,
                duration_min: new_duration,
                classroom: payload.classroom,
                recurrence,
            },
        )
        .await?;

    let groups = match normalized_groups {
        Some(groups) => {
            repo.set_event_groups(id, &effective_subject, &groups)
                .await?;
            groups
        }
        None => repo.find_groups(id).await?,
    };

    tracing::info!(event_id = %id, "Evento actualizado");

    Ok(event_row_to_item(row, groups))
}

#[tracing::instrument(skip(repo), fields(event_id = %id))]
pub async fn delete_event(repo: &dyn EventRepository, id: Uuid) -> Result<DeleteEventResponse, AppError> {
    repo.delete_event(id).await?;

    tracing::info!(event_id = %id, "Evento eliminado");

    Ok(DeleteEventResponse {
        message: "Evento eliminado".into(),
    })
}

#[tracing::instrument(skip(repo), fields(event_id = %id))]
pub async fn get_event(repo: &dyn EventRepository, id: Uuid) -> Result<EventItem, AppError> {
    let row = repo.find_by_id(id).await?.ok_or(AppError::NotFound)?;
    let groups = repo.find_groups(id).await?;
    Ok(event_row_to_item(row, groups))
}

#[tracing::instrument(skip(repo, query))]
pub async fn list_events(
    repo: &dyn EventRepository,
    query: ListEventsQuery,
) -> Result<ListEventsResponse, AppError> {
    let page = query.page.unwrap_or(1).max(1) as i64;
    let limit = query.limit.unwrap_or(20).min(100) as i64;
    let offset = (page - 1) * limit;

    let (rows, total) = repo
        .list_events(query.subject.as_deref(), query.search.as_deref(), offset, limit)
        .await?;

    let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let groups_by_event = fetch_groups_by_event(repo, &ids).await?;

    let events = rows
        .into_iter()
        .map(|r| {
            let groups = groups_by_event.get(&r.id).cloned().unwrap_or_default();
            event_row_with_total_to_item(r, groups)
        })
        .collect();

    Ok(ListEventsResponse { events, total })
}

#[tracing::instrument(skip(repo, query))]
pub async fn list_occurrences(
    repo: &dyn EventRepository,
    query: ListOccurrencesQuery,
) -> Result<ListOccurrencesResponse, AppError> {
    if query.to <= query.from {
        return Err(AppError::BadRequest("'to' debe ser posterior a 'from'".into()));
    }
    if (query.to - query.from).num_days() > MAX_OCCURRENCES_RANGE_DAYS {
        return Err(AppError::BadRequest(format!(
            "El rango entre 'from' y 'to' no puede superar los {MAX_OCCURRENCES_RANGE_DAYS} días"
        )));
    }

    let rows = repo
        .list_occurrences(query.from, query.to, query.subject.as_deref())
        .await?;

    let ids: Vec<Uuid> = {
        let mut ids: Vec<Uuid> = rows.iter().map(|r| r.event_id).collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    };
    let groups_by_event = fetch_groups_by_event(repo, &ids).await?;

    let occurrences = rows
        .into_iter()
        .map(|r| {
            let groups = groups_by_event.get(&r.event_id).cloned().unwrap_or_default();
            occurrence_row_to_item(r, groups)
        })
        .collect();

    Ok(ListOccurrencesResponse { occurrences })
}

async fn fetch_groups_by_event(
    repo: &dyn EventRepository,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<String>>, AppError> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let pairs = repo.find_groups_for_events(ids).await?;
    let mut map: HashMap<Uuid, Vec<String>> = HashMap::new();
    for (event_id, grp) in pairs {
        map.entry(event_id).or_default().push(grp);
    }
    Ok(map)
}

async fn validate_groups_exist(
    repo: &dyn EventRepository,
    subject: &str,
    groups: &[String],
) -> Result<(), AppError> {
    let existing = repo.find_existing_groups(subject, groups).await?;

    let missing: Vec<&String> = groups.iter().filter(|g| !existing.contains(g)).collect();
    if !missing.is_empty() {
        let missing_str = missing
            .into_iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(AppError::BadRequest(format!(
            "Los siguientes grupos no existen para la asignatura '{subject}': {missing_str}"
        )));
    }

    Ok(())
}

fn normalize_groups(groups: Option<Vec<String>>) -> Vec<String> {
    normalize_groups_ref(&groups.unwrap_or_default())
}

fn normalize_groups_ref(groups: &[String]) -> Vec<String> {
    let mut normalized: Vec<String> = groups
        .iter()
        .map(|g| g.trim().to_string())
        .filter(|g| !g.is_empty())
        .collect();
    normalized.sort_unstable();
    normalized.dedup();
    normalized
}

fn validate_duration(
    start: chrono::DateTime<Utc>,
    end: chrono::DateTime<Utc>,
) -> Result<i32, AppError> {
    let minutes = (end - start).num_minutes();
    if minutes <= 0 {
        return Err(AppError::BadRequest(
            "endTime debe ser posterior a startTime".into(),
        ));
    }
    Ok(minutes as i32)
}

fn validate_recurrence(
    start: chrono::DateTime<Utc>,
    recurrence: &RecurrenceInput,
) -> Result<(), AppError> {
    if recurrence.end_date < start.date_naive() {
        return Err(AppError::BadRequest(
            "recurrence.endDate no puede ser anterior a la fecha de startTime".into(),
        ));
    }
    Ok(())
}

fn event_row_to_item(row: EventRow, groups: Vec<String>) -> EventItem {
    EventItem {
        id: row.id,
        title: row.title,
        description: row.description,
        subject: row.subject,
        groups,
        start_time: row.starts_at,
        end_time: row.starts_at + Duration::minutes(row.duration_min as i64),
        classroom: row.classroom,
        recurrence: row
            .recurrence_interval
            .zip(row.recurrence_end_date)
            .map(|(interval, end_date)| RecurrenceInfo { interval, end_date }),
        created_by: row.created_by_email,
        created_at: row.created_at,
    }
}

fn event_row_with_total_to_item(row: EventRowWithTotal, groups: Vec<String>) -> EventItem {
    event_row_to_item(
        EventRow {
            id: row.id,
            title: row.title,
            description: row.description,
            subject: row.subject,
            starts_at: row.starts_at,
            duration_min: row.duration_min,
            classroom: row.classroom,
            recurrence_interval: row.recurrence_interval,
            recurrence_end_date: row.recurrence_end_date,
            created_by_email: row.created_by_email,
            created_at: row.created_at,
        },
        groups,
    )
}

fn occurrence_row_to_item(row: OccurrenceRow, groups: Vec<String>) -> EventOccurrence {
    EventOccurrence {
        event_id: row.event_id,
        title: row.title,
        description: row.description,
        subject: row.subject,
        groups,
        classroom: row.classroom,
        start_time: row.occurrence_starts_at,
        end_time: row.occurrence_starts_at + Duration::minutes(row.duration_min as i64),
        is_recurring: row.is_recurring,
    }
}
