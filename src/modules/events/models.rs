use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Type, PartialEq, Eq)]
#[sqlx(type_name = "recurrence_interval", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum RecurrenceInterval {
    Daily,
    Weekly,
    Biweekly,
    Monthly,
}

/// Payload de recurrencia embebido en `CreateEventRequest`/`UpdateEventRequest`.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceInput {
    pub interval: RecurrenceInterval,
    pub end_date: NaiveDate,
}

/// Payload de `POST /events`.
///
/// `groups` vacío o ausente ⇒ el evento aplica a TODOS los grupos de `subject`.
/// `groups` con valores ⇒ aplica solo a esos grupos concretos de `subject`.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateEventRequest {
    #[validate(length(min = 1, max = 200, message = "El título es obligatorio"))]
    pub title: String,

    #[validate(length(max = 5000, message = "La descripción es demasiado larga"))]
    pub description: Option<String>,

    #[validate(length(min = 1, message = "La asignatura es obligatoria"))]
    pub subject: String,

    #[serde(default)]
    pub groups: Option<Vec<String>>,

    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,

    pub classroom: Option<String>,

    #[validate(nested)]
    pub recurrence: Option<RecurrenceInput>,
}

/// Payload de `PATCH /events/{id}`. Todos los campos son opcionales: solo se
/// actualiza lo que venga presente en el JSON.
///
/// `groups`: si el campo está ausente, no se tocan los grupos existentes; si
/// viene presente (incluso como `[]`), reemplaza por completo la asociación
/// (`[]` = pasa a aplicar a toda la asignatura).
///
/// `recurrence`: si el campo está ausente, no se toca la recurrencia
/// existente; si viene con un valor, reemplaza (o añade) la recurrencia.
/// No existe una forma de "quitar" la recurrencia de un evento ya creado
/// mediante este endpoint: hay que borrarlo y crear uno nuevo sin `recurrence`.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEventRequest {
    #[validate(length(min = 1, max = 200, message = "El título no puede estar vacío"))]
    pub title: Option<String>,

    #[validate(length(max = 5000, message = "La descripción es demasiado larga"))]
    pub description: Option<String>,

    #[validate(length(min = 1, message = "La asignatura no puede estar vacía"))]
    pub subject: Option<String>,

    #[serde(default)]
    pub groups: Option<Vec<String>>,

    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,

    pub classroom: Option<String>,

    #[validate(nested)]
    pub recurrence: Option<RecurrenceInput>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceInfo {
    pub interval: RecurrenceInterval,
    pub end_date: NaiveDate,
}

/// Representación de un evento (la definición, no una ocurrencia concreta).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventItem {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub subject: String,
    /// Vacío ⇒ aplica a toda la asignatura.
    pub groups: Vec<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub classroom: Option<String>,
    pub recurrence: Option<RecurrenceInfo>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct EventRow {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub subject: String,
    pub starts_at: DateTime<Utc>,
    pub duration_min: i32,
    pub classroom: Option<String>,
    pub recurrence_interval: Option<RecurrenceInterval>,
    pub recurrence_end_date: Option<NaiveDate>,
    pub created_by_email: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct EventRowWithTotal {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub subject: String,
    pub starts_at: DateTime<Utc>,
    pub duration_min: i32,
    pub classroom: Option<String>,
    pub recurrence_interval: Option<RecurrenceInterval>,
    pub recurrence_end_date: Option<NaiveDate>,
    pub created_by_email: String,
    pub created_at: DateTime<Utc>,
    pub total: i64,
}

pub struct CreateEventInput {
    pub title: String,
    pub description: Option<String>,
    pub subject: String,
    pub starts_at: DateTime<Utc>,
    pub duration_min: i32,
    pub classroom: Option<String>,
    pub recurrence_interval: Option<RecurrenceInterval>,
    pub recurrence_end_date: Option<NaiveDate>,
    pub created_by: Uuid,
}

#[derive(Default)]
pub struct UpdateEventInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub subject: Option<String>,
    pub starts_at: Option<DateTime<Utc>>,
    pub duration_min: Option<i32>,
    pub classroom: Option<String>,
    /// `None` = no tocar recurrencia. `Some(_)` = fijar esta recurrencia.
    pub recurrence: Option<(RecurrenceInterval, NaiveDate)>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ListEventsQuery {
    pub subject: Option<String>,
    pub search: Option<String>,
    #[validate(range(min = 1, message = "El parámetro 'page' debe ser mayor o igual a 1"))]
    pub page: Option<u32>,
    #[validate(range(
        min = 1,
        max = 100,
        message = "El parámetro 'limit' debe estar entre 1 y 100"
    ))]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListEventsResponse {
    pub events: Vec<EventItem>,
    pub total: i64,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ListOccurrencesQuery {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub subject: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OccurrenceRow {
    pub event_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub subject: String,
    pub classroom: Option<String>,
    pub occurrence_starts_at: DateTime<Utc>,
    pub duration_min: i32,
    pub is_recurring: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventOccurrence {
    pub event_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub subject: String,
    /// Vacío ⇒ aplica a toda la asignatura.
    pub groups: Vec<String>,
    pub classroom: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub is_recurring: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOccurrencesResponse {
    pub occurrences: Vec<EventOccurrence>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteEventResponse {
    pub message: String,
}
