use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::{
        classes::repository::ClassRepository,
        proposals::{
            models::{
                ApproveProposalResponse, Change, ChangeStatus, ChangeType, ChangeWithAuthor,
                CreateChangeInput, CreateProposalRequest, CreateProposalResponse,
                ListOfModifications, ListProposalsResponse, ProposalStatusFilter,
                RejectProposalResponse, ResumedChange,
            },
            repository::ProposalRepository,
        },
    },
};

pub async fn create_proposal(
    repo: &dyn ProposalRepository,
    class_repo: &dyn ClassRepository,
    payload: CreateProposalRequest,
    proposed_by: Uuid,
) -> Result<CreateProposalResponse, AppError> {
    let change = match payload {
        CreateProposalRequest::Create(changes) => {
            class_repo
                .upsert_subject_group(&changes.subject, &changes.grp)
                .await?;
            repo.create_change(CreateChangeInput {
                proposed_by,
                change_type: ChangeType::Create,
                session_id: None,
                subject: Some(changes.subject),
                grp: Some(changes.grp),
                new_starts_at: Some(changes.new_starts_at),
                new_duration: Some(changes.new_duration),
                new_classroom: Some(changes.new_classroom),
                prev_starts_at: None,
                prev_duration: None,
                prev_classroom: None,
                status: ChangeStatus::Pending,
            })
            .await?
        }
        CreateProposalRequest::Modify(changes) => {
            let session = class_repo
                .find_by_id(changes.session_id)
                .await?
                .ok_or(AppError::NotFound)?;

            repo.create_change(CreateChangeInput {
                proposed_by,
                change_type: ChangeType::Modify,
                session_id: Some(changes.session_id),
                subject: Some(session.subject.clone()),
                grp: Some(session.grp.clone()),
                new_starts_at: changes.new_starts_at,
                new_duration: changes.new_duration,
                new_classroom: changes.new_classroom,
                prev_starts_at: Some(session.starts_at),
                prev_duration: Some(session.duration_min),
                prev_classroom: session.classroom,
                status: ChangeStatus::Pending,
            })
            .await?
        }
        CreateProposalRequest::Delete(changes) => {
            let session = class_repo
                .find_by_id(changes.session_id)
                .await?
                .ok_or(AppError::NotFound)?;

            repo.create_change(CreateChangeInput {
                proposed_by,
                change_type: ChangeType::Delete,
                session_id: Some(changes.session_id),
                subject: Some(session.subject.clone()),
                grp: Some(session.grp.clone()),
                prev_starts_at: Some(session.starts_at),
                prev_duration: Some(session.duration_min),
                prev_classroom: session.classroom,
                new_starts_at: None,
                new_duration: None,
                new_classroom: None,
                status: ChangeStatus::Pending,
            })
            .await?
        }
    };

    Ok(CreateProposalResponse::from(change))
}

pub async fn approve_proposal(
    proposal_repo: &dyn ProposalRepository,
    class_repo: &dyn ClassRepository,
    change_id: Uuid,
    approved_by: Uuid,
) -> Result<ApproveProposalResponse, AppError> {
    let change = proposal_repo.find_by_id(change_id).await?;
    if change.change_status != ChangeStatus::Pending {
        return Err(AppError::Conflict("Change is not pending".into()));
    }
    match change.change_type {
        ChangeType::Create => approve_create(class_repo, change, approved_by).await?,
        ChangeType::Delete => approve_delete(class_repo, change).await?,
        ChangeType::Modify => approve_modify(class_repo, change).await?,
    }

    proposal_repo.approve(change_id).await?;

    Ok(ApproveProposalResponse {
        id: (change_id),
        status: (super::models::ChangeStatus::Approved),
    })
}

async fn approve_create(
    class_repo: &dyn ClassRepository,
    change: Change,
    approved_by: Uuid,
) -> Result<(), AppError> {
    let subject = change
        .subject
        .as_deref()
        .ok_or(AppError::BadRequest("Missing subject".into()))?;

    let grp = change
        .grp
        .as_deref()
        .ok_or(AppError::BadRequest("Missing grp".into()))?;

    let starts_at = change
        .new_starts_at
        .ok_or(AppError::BadRequest("Missing starts_at".into()))?;

    let duration = change
        .new_duration
        .ok_or(AppError::BadRequest("Missing duration".into()))?;

    let classroom = change.new_classroom.as_deref();

    class_repo
        .create_session(subject, grp, starts_at, duration, classroom, approved_by)
        .await?;

    Ok(())
}

/// Resuelve el UUID de la sesión a partir de un change.
///
/// Intenta primero usar `session_id` directamente. Si está a NULL (el scraper
/// corrió entre la propuesta y la aprobación y lo puso a NULL vía ON DELETE SET NULL),
/// cae a buscar la sesión por clave natural (subject, grp, prev_starts_at).
async fn resolve_session_id(
    class_repo: &dyn ClassRepository,
    change: &Change,
) -> Result<Uuid, AppError> {
    if let Some(id) = change.session_id {
        return Ok(id);
    }

    let subject = change.subject.as_deref().ok_or_else(|| {
        AppError::BadRequest(
            "No se puede resolver la sesión: session_id es NULL y no hay clave natural".into(),
        )
    })?;
    let grp = change.grp.as_deref().ok_or_else(|| {
        AppError::BadRequest(
            "No se puede resolver la sesión: session_id es NULL y no hay clave natural".into(),
        )
    })?;
    let prev_starts_at = change.prev_starts_at.ok_or_else(|| {
        AppError::BadRequest(
            "No se puede resolver la sesión: session_id es NULL y no hay clave natural".into(),
        )
    })?;

    class_repo
        .find_by_natural_key(subject, grp, prev_starts_at)
        .await?
        .ok_or(AppError::NotFound)
}

async fn approve_delete(class_repo: &dyn ClassRepository, change: Change) -> Result<(), AppError> {
    let session_id = resolve_session_id(class_repo, &change).await?;
    class_repo.delete_session(session_id).await?;
    Ok(())
}

async fn approve_modify(class_repo: &dyn ClassRepository, change: Change) -> Result<(), AppError> {
    let session_id = resolve_session_id(class_repo, &change).await?;

    // subject y grp no se modifican en un modify (son la clave de identidad de la sesión,
    // no campos que el change quiera cambiar)
    let starts_at = change.new_starts_at;
    let duration_min = change.new_duration;
    let classroom = change.new_classroom.as_deref();

    class_repo
        .update_session(session_id, None, None, starts_at, duration_min, classroom)
        .await?;

    Ok(())
}

pub async fn reject_proposal(
    proposal_repo: &dyn ProposalRepository,
    change_id: Uuid,
) -> Result<RejectProposalResponse, AppError> {
    let change = proposal_repo.find_by_id(change_id).await?;
    if change.change_status != ChangeStatus::Pending {
        return Err(AppError::Conflict("Change is not pending".into()));
    }

    proposal_repo.reject(change_id).await?;

    Ok(RejectProposalResponse {
        id: (change_id),
        status: (super::models::ChangeStatus::Rejected),
    })
}

pub async fn list_proposals(
    proposal_repo: &dyn ProposalRepository,
    status: Option<ProposalStatusFilter>,
    page: u32,
    limit: u32,
) -> Result<ListProposalsResponse, AppError> {
    let offset = (page - 1) * limit;

    let status_filter = match status {
        None | Some(ProposalStatusFilter::All) => None,
        Some(ProposalStatusFilter::Pending) => Some(ChangeStatus::Pending),
        Some(ProposalStatusFilter::Approved) => Some(ChangeStatus::Approved),
        Some(ProposalStatusFilter::Rejected) => Some(ChangeStatus::Rejected),
    };

    let (changes, total) = proposal_repo
        .list_by_status(status_filter, offset, limit)
        .await?;

    let data = changes.into_iter().map(change_to_resumed).collect();

    Ok(ListProposalsResponse {
        data,
        total,
        page,
        limit,
    })
}

pub async fn list_my_proposals(
    proposal_repo: &dyn ProposalRepository,
    user_id: Uuid,
    page: u32,
    limit: u32,
) -> Result<ListProposalsResponse, AppError> {
    let offset = (page - 1) * limit;

    let (changes, total) = proposal_repo
        .list_by_proposer(user_id, offset, limit)
        .await?;

    let data = changes.into_iter().map(change_to_resumed).collect();

    Ok(ListProposalsResponse {
        data,
        total,
        page,
        limit,
    })
}

/// Convierte un `ChangeWithAuthor` a un `ResumedChange`, que es la forma en la que se devuelve en la lista de propuestas.
fn change_to_resumed(c: ChangeWithAuthor) -> ResumedChange {
    ResumedChange {
        id: c.id,
        action: c.change_type,
        class_id: c.session_id,
        old: ListOfModifications {
            subject: None,
            grp: None,
            starts_at: c.prev_starts_at,
            duration: c.prev_duration,
            classroom: c.prev_classroom,
        },
        new: ListOfModifications {
            subject: c.subject,
            grp: c.grp,
            starts_at: c.new_starts_at,
            duration: c.new_duration,
            classroom: c.new_classroom,
        },
        status: c.change_status,
        author: c.author_email,
        created_at: c.proposed_at,
    }
}
