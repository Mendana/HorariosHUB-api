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
