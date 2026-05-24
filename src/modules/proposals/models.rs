use crate::utils::validators::validate_multiple_of_30;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProposalResponse {
    pub id: Uuid,
    pub status: ChangeStatus,
    pub created_at: DateTime<Utc>,
}

impl From<Change> for CreateProposalResponse {
    fn from(c: Change) -> Self {
        Self {
            id: c.id,
            status: c.change_status,
            created_at: c.proposed_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveProposalResponse {
    pub id: Uuid,
    pub status: ChangeStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectProposalResponse {
    pub id: Uuid,
    pub status: ChangeStatus,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", tag = "changeType", content = "changes")]
pub enum CreateProposalRequest {
    Create(CreateChanges),
    Modify(ModifyChanges),
    Delete(DeleteChanges),
}

impl CreateProposalRequest {
    pub fn change_type(&self) -> &str {
        match self {
            CreateProposalRequest::Create(_) => "create",
            CreateProposalRequest::Modify(_) => "modify",
            CreateProposalRequest::Delete(_) => "delete",
        }
    }
}

impl Validate for CreateProposalRequest {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            CreateProposalRequest::Create(c) => c.validate(),
            CreateProposalRequest::Modify(m) => {
                m.validate()?;

                if let Err(e) = validate_one_attribute_is_present(m) {
                    let mut errors = validator::ValidationErrors::new();
                    errors.add("changes", e);
                    return Err(errors);
                }

                Ok(())
            }
            CreateProposalRequest::Delete(_) => Ok(()),
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateChanges {
    #[validate(length(
        min = 1,
        message = "La asignatura tiene que estar presente si se trata de una creación."
    ))]
    pub subject: String,
    #[validate(length(
        min = 1,
        message = "El grupo tiene que estar presente si se trata de una creación."
    ))]
    pub grp: String,

    pub new_starts_at: DateTime<Utc>,
    #[validate(
        range(min = 30, message = "La duración mínima de la clase es de 30 minutos"),
        custom(function = "validate_multiple_of_30")
    )]
    pub new_duration: i32,
    #[validate(length(min = 1, message = "La clase tiene que ser valida."))]
    pub new_classroom: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ModifyChanges {
    pub session_id: Uuid,
    pub new_starts_at: Option<DateTime<Utc>>,
    #[validate(
        range(min = 30, message = "La duración mínima de la clase es de 30 minutos"),
        custom(function = "validate_multiple_of_30")
    )]
    pub new_duration: Option<i32>,
    #[validate(length(min = 1, message = "La clase tiene que tener al menos un caracter"))]
    pub new_classroom: Option<String>,
}

fn validate_one_attribute_is_present(
    changes: &ModifyChanges,
) -> Result<(), validator::ValidationError> {
    if changes.new_duration.is_none()
        && changes.new_starts_at.is_none()
        && changes.new_classroom.is_none()
    {
        Err(validator::ValidationError::new(
            "Al menos algo tiene que cambiar",
        ))
    } else {
        Ok(())
    }
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DeleteChanges {
    pub session_id: Uuid,
}

/// A diferencia de `ChangeStatus`, incluye la variante `All` para no filtrar.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProposalStatusFilter {
    Pending,
    Approved,
    Rejected,
    All,
}

#[derive(Debug, Deserialize)]
pub struct ListProposalsQuery {
    pub status: Option<ProposalStatusFilter>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListProposalsResponse {
    pub data: Vec<ResumedChange>,
    pub total: u32,
    pub page: u32,
    pub limit: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumedChange {
    pub id: Uuid,
    pub action: ChangeType,
    pub class_id: Option<Uuid>,
    pub old: ListOfModifications,
    pub new: ListOfModifications,
    pub status: ChangeStatus,
    pub author: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOfModifications {
    pub subject: Option<String>,
    pub grp: Option<String>,
    pub starts_at: Option<DateTime<Utc>>,
    pub duration: Option<i32>,
    pub classroom: Option<String>,
}

#[derive(Debug, sqlx::Type, Deserialize, Serialize)]
#[sqlx(type_name = "change_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    Create,
    Modify,
    Delete,
}

#[derive(Debug, Clone, sqlx::Type, Serialize, PartialEq, Deserialize)]
#[sqlx(type_name = "change_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ChangeStatus {
    Pending,
    Approved,
    Rejected,
}

pub struct Change {
    pub id: Uuid,
    pub proposed_by: Uuid,
    pub session_id: Option<Uuid>,
    pub subject: Option<String>,
    pub grp: Option<String>,
    pub change_type: ChangeType,
    pub change_status: ChangeStatus,
    pub prev_starts_at: Option<DateTime<Utc>>,
    pub prev_duration: Option<i32>,
    pub new_starts_at: Option<DateTime<Utc>>,
    pub new_duration: Option<i32>,
    pub proposed_at: DateTime<Utc>,
    pub prev_classroom: Option<String>,
    pub new_classroom: Option<String>,
}

/// Igual que `Change` pero con el email del autor ya resuelto (resultado de JOIN con users).
pub struct ChangeWithAuthor {
    pub id: Uuid,
    pub proposed_by: Uuid,
    pub session_id: Option<Uuid>,
    pub subject: Option<String>,
    pub grp: Option<String>,
    pub change_type: ChangeType,
    pub change_status: ChangeStatus,
    pub prev_starts_at: Option<DateTime<Utc>>,
    pub prev_duration: Option<i32>,
    pub new_starts_at: Option<DateTime<Utc>>,
    pub new_duration: Option<i32>,
    pub proposed_at: DateTime<Utc>,
    pub prev_classroom: Option<String>,
    pub new_classroom: Option<String>,
    pub author_email: String,
}

pub struct CreateChangeInput {
    pub proposed_by: Uuid,
    pub change_type: ChangeType,
    pub session_id: Option<Uuid>,
    pub subject: Option<String>,
    pub grp: Option<String>,
    pub new_starts_at: Option<DateTime<Utc>>,
    pub new_duration: Option<i32>,
    pub new_classroom: Option<String>,
    pub prev_starts_at: Option<DateTime<Utc>>,
    pub prev_duration: Option<i32>,
    pub prev_classroom: Option<String>,
}
