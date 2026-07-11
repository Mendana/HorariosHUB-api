use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;
use validator::Validate;

/// Payload del endpoint `POST /api/classes`
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateClassRequest {
    pub group_id: Option<Uuid>,

    #[validate(length(min = 1, message = "El nombre de la clase es obligatorio"))]
    pub subject: Option<String>,

    #[validate(length(min = 1, message = "El tipo de clase es obligatorio"))]
    pub subject_type: Option<String>,

    pub classroom: Option<String>,

    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
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

/// Payload del endpoint `PATCH /api/classes/{id}`
///
/// Permite actualizar los campos de una clase
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateClassRequest {
    pub group_id: Option<Uuid>,

    #[validate(length(min = 1, message = "El nombre de la clase no puede estar vacío"))]
    pub subject: Option<String>,

    pub subject_type: Option<String>,

    pub classroom: Option<String>,

    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

/// Respuesta de `DELETE /api/classes/{id}`
#[derive(Debug, Serialize)]
pub struct DeleteClassResponse {
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize, Type, Clone, PartialEq)]
#[sqlx(type_name = "session_source", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SessionSource {
    Scraper,
    Manual,
}

/// Payload del endpoint `GET /api/classes`
#[derive(Debug, Deserialize, Validate)]
pub struct ListClassesQueryParams {
    pub search: Option<String>,
    pub week: Option<String>,
    pub sort: Option<ListClassesSortOption>,
    pub dir: Option<ListClassesSortDirection>,
    #[validate(range(min = 1, message = "El parámetro 'page' debe ser mayor o igual a 1"))]
    pub page: Option<u32>,
    #[validate(range(
        min = 1,
        max = 100,
        message = "El parámetro 'limit' debe estar entre 1 y 100"
    ))]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListClassesSortOption {
    Name,
    #[serde(rename = "type")]
    Type,
    Date,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListClassesSortDirection {
    Asc,
    Desc,
}

impl FromStr for ListClassesSortOption {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "name" => Ok(ListClassesSortOption::Name),
            "type" => Ok(ListClassesSortOption::Type),
            "date" => Ok(ListClassesSortOption::Date),
            _ => Err(()),
        }
    }
}

impl FromStr for ListClassesSortDirection {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "asc" => Ok(ListClassesSortDirection::Asc),
            "desc" => Ok(ListClassesSortDirection::Desc),
            _ => Err(()),
        }
    }
}

/// Parámetros internos para `ClassRepository::list_sessions`
pub struct ListSessionsParams<'a> {
    pub search: Option<&'a str>,
    pub week_start: Option<DateTime<Utc>>,
    pub week_end: Option<DateTime<Utc>>,
    pub order_col: &'a str,
    pub order_dir: &'a str,
    pub limit: i64,
    pub offset: i64,
}

/// Response del endpoint `GET /api/classes`
#[derive(Debug, Serialize)]
pub struct ListClassesResponse {
    pub classes: Vec<ClassItem>,
    pub total: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassItem {
    pub id: Uuid,
    pub subject: String,
    pub subject_type: String,
    pub classroom: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ClassItemRow {
    pub id: Uuid,
    pub subject: String,
    pub grp: String,
    pub classroom: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub duration_min: i32,
    pub total: i64,
}
