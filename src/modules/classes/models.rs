use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;
use validator::Validate;

/// Payload del endpoint `POST /api/classes`
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateClassRequest {
    #[validate(length(min = 1, message = "El nombre de la clase es obligatorio"))]
    pub name: String,

    #[validate(length(min = 1, message = "El tipo de clase es obligatorio"))]
    pub r#type: String,

    pub classroom: Option<String>,

    pub date: DateInput,

    #[validate(length(min = 1, message = "La hora de inicio es obligatoria"))]
    pub start_time: String,

    #[validate(
        range(min = 30, message = "La duración mínima de la clase es de 30 minutos"),
        custom(function = "validate_multiple_of_30")
    )]
    pub duration_minutes: i32,
}

fn validate_multiple_of_30(duration: i32) -> Result<(), validator::ValidationError> {
    if duration % 30 != 0 {
        return Err(validator::ValidationError::new(
            "duration_minutes must be a multiple of 30",
        ));
    }
    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub subject: String,
    pub grp: String,
    pub starts_at: DateTime<Utc>,
    pub duration_min: i32,
    pub classroom: Option<String>,
    pub source: SessionSource,
    pub created_by: Option<Uuid>,
    pub is_overridden: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateClassResponse {
    pub id: Uuid,
    pub name: String,
    pub r#type: String,
    pub classroom: Option<String>,
    pub date: String,
    pub start_time: String,
    pub end_time: String,
    pub duration_minutes: i32,
}

#[derive(Debug, Deserialize, Serialize, Type, Clone, PartialEq)]
#[sqlx(type_name = "session_source", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SessionSource {
    Scraper,
    Manual,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateInput {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}
