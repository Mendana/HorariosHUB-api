use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct CatalogResponse {
    pub subjects: Vec<SubjectEntry>,
}

#[derive(Debug, Serialize)]
pub struct SubjectEntry {
    pub code: String,
    pub groups: Vec<GroupEntry>,
}

#[derive(Debug, Serialize)]
pub struct GroupEntry {
    pub id: Uuid,
    pub name: String,
    pub selected: bool,
}

#[derive(Debug, Serialize)]
pub struct SubjectGroupRow {
    pub id: Uuid,
    pub subject: String,
    pub grp: String,
    pub selected: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserSelectionRequest {
    pub groups: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct UserSelectionResponse {
    pub message: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct AutoSelectResponse {
    pub job_id: Uuid,
    pub status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoSelectStatus {
    Processing,
    Completed,
    Failed,
}

impl AutoSelectStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AutoSelectStatus::Processing => "processing",
            AutoSelectStatus::Completed => "completed",
            AutoSelectStatus::Failed => "failed",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AutoSelectStatusResponse {
    pub job_id: Uuid,
    pub status: AutoSelectStatus,
    pub groups_selected: Option<i32>,
    pub error: Option<String>,
}

pub struct JobStatusRow {
    pub id: Uuid,
    pub status: String,
    pub groups_selected: Option<i32>,
    pub error: Option<String>,
}
