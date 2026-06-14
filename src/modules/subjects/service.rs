use std::{sync::Arc, time::Duration};

use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::subjects::{
        models::{
            AutoSelectResponse, AutoSelectStatus, AutoSelectStatusResponse, CatalogResponse,
            GroupEntry, GroupEntryWithoutSelection, JobStatusRow, SubjectEntry,
            UserSelectionResponse,
        },
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

pub async fn set_user_selection_destructive(
    repo: &dyn SubjectRepository,
    user_id: Uuid,
    groups_ids: Vec<Uuid>,
) -> Result<UserSelectionResponse, AppError> {
    let groups_len = groups_ids.len();

    repo.set_user_selection_destructive(user_id, groups_ids)
        .await?;

    Ok(UserSelectionResponse {
        message: "Selección guardada".to_string(),
        count: groups_len,
    })
}

pub async fn auto_select_subjects(
    repo: Arc<dyn SubjectRepository>,
    semaphore: Arc<Semaphore>,
    user_id: Uuid,
    user_email: &str,
    scraper_url: &str,
) -> Result<AutoSelectResponse, AppError> {
    let uo_username = user_email.split('@').next().unwrap_or("");

    if !is_uo_username(uo_username) {
        return Err(AppError::BadRequest(
            "El nombre de usuario debe tener el formato 'uo' seguido de entre 4 y 6 dígitos (ej. uo123456)"
                .to_string(),
        ));
    }

    // Verificar que no haya ya un job activo para este usuario
    if repo.get_active_job_for_user(user_id).await?.is_some() {
        return Err(AppError::Conflict(
            "Ya hay un proceso de auto-selección en curso para este usuario".to_string(),
        ));
    }

    // Limitar el número de procesos simultáneos a nivel global
    let permit = match semaphore.try_acquire_owned() {
        Ok(p) => p,
        Err(_) => return Err(AppError::TooManyRequests),
    };

    let job_id = repo.create_auto_select_job(user_id).await?;

    let repo_clone = repo.clone();
    let full_url = format!("{scraper_url}/auto-select/{uo_username}");

    tokio::spawn(async move {
        let _permit = permit; // El semáforo se libera al salir de este bloque

        let result = async {
            let groups = fetch_groups_for_uo(&full_url).await?;
            let count = repo_clone
                .set_user_selection_by_subject_grp(user_id, groups)
                .await?;
            Ok::<i32, AppError>(count)
        }
        .await;

        match result {
            Ok(count) => {
                tracing::info!(job_id = %job_id, groups = count, "Auto-select completado");
                if let Err(e) = repo_clone.complete_auto_select_job(job_id, count).await {
                    tracing::error!(job_id = %job_id, error = ?e, "Error al marcar job como completado");
                }
            }
            Err(e) => {
                tracing::error!(job_id = %job_id, error = ?e, "Error durante el auto-select");
                if let Err(e2) = repo_clone
                    .fail_auto_select_job(job_id, &e.to_string())
                    .await
                {
                    tracing::error!(job_id = %job_id, error = ?e2, "Error al marcar job como fallido");
                }
            }
        }
    });

    Ok(AutoSelectResponse {
        job_id,
        status: "processing".to_string(),
    })
}

pub async fn auto_select_subjects_status(
    repo: &dyn SubjectRepository,
    user_id: Uuid,
) -> Result<AutoSelectStatusResponse, AppError> {
    let JobStatusRow {
        id: job_id,
        status,
        groups_selected,
        error,
    } = repo
        .get_latest_job_for_user(user_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let status = match status.as_str() {
        "processing" => AutoSelectStatus::Processing,
        "completed" => AutoSelectStatus::Completed,
        "failed" => AutoSelectStatus::Failed,
        other => {
            return Err(AppError::Internal(anyhow::anyhow!(
                "Estado desconocido en auto_select_jobs: {other}"
            )));
        }
    };

    Ok(AutoSelectStatusResponse {
        job_id,
        status,
        groups_selected,
        error,
    })
}

pub async fn get_all_subjects_catalog(
    repo: &dyn SubjectRepository,
) -> Result<Vec<String>, AppError> {
    let rows = repo.get_all_subjects_catalog().await?;

    Ok(rows)
}

pub async fn get_all_groups_per_subject(
    repo: &dyn SubjectRepository,
    subject: &str,
) -> Result<Vec<GroupEntryWithoutSelection>, AppError> {
    let groups = repo.get_all_groups_per_subject(subject).await?;
    if groups.is_empty() {
        return Err(AppError::NotFound);
    }
    Ok(groups)
}

fn is_uo_username(username: &str) -> bool {
    if !username.starts_with("uo") {
        return false;
    }
    let digits = &username[2..];
    (4..=6).contains(&digits.len()) && digits.chars().all(|c| c.is_ascii_digit())
}

/// Llama al scraper de auto-select y devuelve la lista de grupos del alumno.
///
/// # Formato esperado de respuesta
/// CSV con una línea por grupo: `subject,grp` (ej. `AL,T.1`)
/// TODO: Confirmar formato exacto cuando todo esté listo
async fn fetch_groups_for_uo(url: &str) -> Result<Vec<(String, String)>, AppError> {
    tracing::info!(url, "Llamando al scraper de auto-select");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60 * 30))
        .build()
        .map_err(|e| AppError::Internal(e.into()))?;

    let response = client.post(url).send().await.map_err(|e| {
        AppError::Internal(anyhow::anyhow!(
            "No se pudo conectar al scraper de auto-select en {url}: {e}"
        ))
    })?;

    if !response.status().is_success() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "El scraper de auto-select respondió con error HTTP {}: {}",
            response.status(),
            response.text().await.unwrap_or_default()
        )));
    }

    let body = response
        .text()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    tracing::info!(
        bytes = body.len(),
        "Respuesta recibida del scraper de auto-select"
    );

    parse_groups_response(&body)
}

fn parse_groups_response(body: &str) -> Result<Vec<(String, String)>, AppError> {
    let mut groups = Vec::new();

    for (line_num, line) in body.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.splitn(2, ',');
        let subject = parts.next().map(str::trim).unwrap_or("").to_string();
        let grp = parts.next().map(str::trim).unwrap_or("").to_string();

        if subject.is_empty() || grp.is_empty() {
            tracing::warn!(
                line = line_num + 1,
                raw = line,
                "Línea malformada en respuesta del scraper de auto-select, ignorando"
            );
            continue;
        }

        groups.push((subject, grp));
    }

    Ok(groups)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uo_username_valido() {
        assert!(is_uo_username("uo1234"));
        assert!(is_uo_username("uo12345"));
        assert!(is_uo_username("uo123456"));
    }

    #[test]
    fn uo_username_invalido() {
        assert!(!is_uo_username("infmatprimero"));
        assert!(!is_uo_username("uo")); // sin dígitos
        assert!(!is_uo_username("uo123")); // demasiado corto (3 dígitos)
        assert!(!is_uo_username("uo1234567")); // demasiado largo (7 dígitos)
        assert!(!is_uo_username("uo123abc")); // contiene letras
        assert!(!is_uo_username("admin"));
    }

    #[test]
    fn parse_grupos_correcto() {
        let body = "AL,T.1\nALG,T.2\n\n# comentario\nFIS,P.3\n";
        let groups = parse_groups_response(body).unwrap();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0], ("AL".to_string(), "T.1".to_string()));
        assert_eq!(groups[1], ("ALG".to_string(), "T.2".to_string()));
        assert_eq!(groups[2], ("FIS".to_string(), "P.3".to_string()));
    }

    #[test]
    fn parse_grupos_lineas_malformadas_ignoradas() {
        let body = "AL,T.1\nmalformada\n,sinsubject\nALG,T.2\n";
        let groups = parse_groups_response(body).unwrap();
        assert_eq!(groups.len(), 2);
    }
}
