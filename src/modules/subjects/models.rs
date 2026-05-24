use serde::Serialize;
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
