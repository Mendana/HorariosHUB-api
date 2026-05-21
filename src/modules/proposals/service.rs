use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::{
        classes::repository::ClassRepository,
        proposals::{
            models::{
                ApproveProposalResponse, Change, ChangeStatus, ChangeType, CreateChangeInput,
                CreateProposalRequest, CreateProposalResponse,
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
                subject: None,
                grp: None,
                new_starts_at: changes.new_starts_at,
                new_duration: changes.new_duration,
                new_classroom: changes.new_classroom,
                prev_starts_at: Some(session.starts_at),
                prev_duration: Some(session.duration_min),
                prev_classroom: session.classroom,
            })
            .await?
        }
        CreateProposalRequest::Delete(changes) => {
            class_repo
                .find_by_id(changes.session_id)
                .await?
                .ok_or(AppError::NotFound)?;
            repo.create_change(CreateChangeInput {
                proposed_by,
                change_type: ChangeType::Delete,
                session_id: Some(changes.session_id),
                subject: None,
                grp: None,
                new_starts_at: None,
                new_duration: None,
                prev_starts_at: None,
                prev_duration: None,
                new_classroom: None,
                prev_classroom: None,
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

async fn approve_delete(class_repo: &dyn ClassRepository, change: Change) -> Result<(), AppError> {
    let session_id = change
        .session_id
        .ok_or(AppError::BadRequest("Missing session_id".into()))?;

    class_repo.delete_session(session_id).await?;

    Ok(())
}

async fn approve_modify(class_repo: &dyn ClassRepository, change: Change) -> Result<(), AppError> {
    let session_id = change
        .session_id
        .ok_or(AppError::BadRequest("Missing session_id".into()))?;

    let subject = change.subject.as_deref();

    let grp = change.grp.as_deref();

    let starts_at = change.new_starts_at.or(change.prev_starts_at);

    let duration_min = change.new_duration.or(change.prev_duration);

    let classroom_owned = change.new_classroom.or(change.prev_classroom);

    let classroom = classroom_owned.as_deref();

    class_repo
        .update_session(session_id, subject, grp, starts_at, duration_min, classroom)
        .await?;

    Ok(())
}
