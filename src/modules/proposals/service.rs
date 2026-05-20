use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::{
        classes::repository::ClassRepository,
        proposals::{
            models::{
                ChangeType, CreateChangeInput, CreateProposalRequest, CreateProposalResponse,
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
