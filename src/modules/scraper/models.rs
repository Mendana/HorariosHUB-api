use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::proposals::models::ChangeType;

/// Fila del CSV del ejecutable .NET
#[derive(Debug, Deserialize)]
pub struct CsvRow {
    #[serde(rename = "Day")]
    pub day: String, // "09/11/2024"

    #[serde(rename = "Start")]
    pub start: String, // "09:00"

    #[serde(rename = "End")]
    pub end: String, // "10:00"

    #[serde(rename = "Subject")]
    pub subject: String, // "AL.T.1"

    #[serde(rename = "Room")]
    pub room: Option<String>, // "A-2-01"
}

/// Sesión parseada y validada para trabajar
#[derive(Debug, Clone)]
pub struct ParsedSession {
    pub subject: String, // "AL"
    pub grp: String,     // "T.1"
    pub starts_at: DateTime<Utc>,
    pub duration_min: i32,
    pub classroom: Option<String>,
}

/// Resultado del algoritmo de sincronización
///
/// - sessions_from_changes: create aprobados aplicados
/// - changes_applied: modify/delete aprobados aplicados
/// - changes_ignored: modify/delete sin sesión de referencia (datos insuficientes en el change)
/// - overrides_expired: modify/delete aprobados cuya sesión ya no existe → archivados
/// - pending_rejected: pending rechazados por huérfanos
/// - rejected_archived: rejected movidos a histórico
#[derive(Debug, Default)]
pub struct SyncResult {
    pub sessions_inserted: usize,
    pub sessions_from_changes: usize,
    pub changes_applied: usize,
    pub changes_ignored: usize,
    pub overrides_expired: usize,
    pub pending_rejected: usize,
    pub rejected_archived: usize,
    pub aborted: bool,
    pub abort_reason: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ApprovedChange {
    pub id: Uuid,
    pub change_type: ChangeType,
    pub subject: Option<String>,
    pub grp: Option<String>,
    pub prev_starts_at: Option<DateTime<Utc>>,
    pub prev_duration: Option<i32>,
    pub prev_classroom: Option<String>,
    pub new_starts_at: Option<DateTime<Utc>>,
    pub new_duration: Option<i32>,
    pub new_classroom: Option<String>,
    pub proposed_by: Uuid,
    pub proposed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResponse {
    pub message: String,
    pub sessions_inserted: usize,
    pub sessions_from_changes: usize,
    pub changes_applied: usize,
    pub changes_ignored: usize,
    pub overrides_expired: usize,
    pub pending_rejected: usize,
    pub rejected_archived: usize,
}

impl From<SyncResult> for SyncResponse {
    fn from(r: SyncResult) -> Self {
        Self {
            message: "Sincronización completada".into(),
            sessions_inserted: r.sessions_inserted,
            sessions_from_changes: r.sessions_from_changes,
            changes_applied: r.changes_applied,
            changes_ignored: r.changes_ignored,
            overrides_expired: r.overrides_expired,
            pending_rejected: r.pending_rejected,
            rejected_archived: r.rejected_archived,
        }
    }
}
