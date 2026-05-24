use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::subjects::{
        models::{CatalogResponse, GroupEntry, SubjectEntry},
        repository::SubjectRepository,
    },
};

pub async fn get_catalog_by_user(
    repo: &dyn SubjectRepository,
    user_id: Uuid,
) -> Result<CatalogResponse, AppError> {
    let rows = repo.get_catalog_by_user(user_id).await?;

    let mut catalog = CatalogResponse {
        subjects: Vec::new(),
    };

    for row in rows {
        if let Some(subject) = catalog.subjects.iter_mut().find(|s| s.code == row.subject) {
            subject.groups.push(GroupEntry {
                id: row.id,
                name: row.grp,
                selected: row.selected,
            });
        } else {
            catalog.subjects.push(SubjectEntry {
                code: row.subject,
                groups: vec![GroupEntry {
                    id: row.id,
                    name: row.grp,
                    selected: row.selected,
                }],
            });
        }
    }

    Ok(catalog)
}
