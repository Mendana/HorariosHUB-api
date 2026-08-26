use std::time::Duration;

use crate::{
    errors::AppError,
    modules::{
        notifications::{
            models::ScraperConflictInfo, repository::NotificationRepository,
            service as notifications_service, service::EmailQueue,
        },
        proposals::models::ChangeType,
        scraper::{
            models::{ApprovedChange, SyncResult},
            parser::parse_csv,
            repository::ScraperRepository,
        },
    },
};

pub async fn fetch_csv_from_scraper(scraper_url: &str) -> Result<String, AppError> {
    tracing::info!(url = scraper_url, "Llamando al servicio del scraper");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60 * 30))
        .build()
        .map_err(|e| AppError::Internal(e.into()))?;

    let response = client.post(scraper_url).send().await.map_err(|e| {
        tracing::error!(url = scraper_url, error = ?e, "No se pudo conectar al scraper");
        AppError::Internal(anyhow::anyhow!(
            "No se pudo conectar al scraper en {scraper_url}: {e}"
        ))
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        tracing::error!(url = scraper_url, status = %status, body = %body, "El scraper respondió con error HTTP");
        return Err(AppError::Internal(anyhow::anyhow!(
            "El scraper respondió con un error HTTP {status}: {body}"
        )));
    }

    let csv = response
        .text()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    tracing::info!(bytes = csv.len(), "CSV recibido del scraper");
    Ok(csv)
}

/// Algoritmo completo de sincronización.
///
/// # Flujo
/// 1. Adquirir lock
/// 2. Realizar petición al scraper y parsear CSV
/// 3. Validar volumen mínimo
/// 4. Borrar todas las sesiones e insertar las del scraper
/// 5. Aplicar cambios aprobados
/// 6. Archivar rejected y pending huérfanos
/// 7. Liberar lock
pub async fn run_sync(
    repo: &dyn ScraperRepository,
    notifications_repo: &dyn NotificationRepository,
    email_queue: &EmailQueue,
    scraper_url: &str,
    min_sessions: usize,
    hostname: &str,
) -> Result<SyncResult, AppError> {
    // 1. Adquirir lock
    let acquired = repo.acquire_lock(hostname).await?;
    if !acquired {
        tracing::warn!("Sincronización abortada - ya hay otra en curso");
        return Ok(SyncResult {
            aborted: true,
            abort_reason: Some("Ya hay una sincronización en curso".into()),
            ..Default::default()
        });
    }

    tracing::info!("Lock adquirido, iniciando sincronización");

    let result = run_sync_inner(
        repo,
        notifications_repo,
        email_queue,
        scraper_url,
        min_sessions,
    )
    .await;

    if let Err(e) = repo.release_lock().await {
        tracing::error!("No se pudo liberar el lock del scraper: {:?}", e);
    }

    result
}

async fn run_sync_inner(
    repo: &dyn ScraperRepository,
    notifications_repo: &dyn NotificationRepository,
    email_queue: &EmailQueue,
    scraper_url: &str,
    min_sessions: usize,
) -> Result<SyncResult, AppError> {
    //2. Ejecutar el binario y parsear CSV
    let csv = fetch_csv_from_scraper(scraper_url).await?;
    let sessions = parse_csv(&csv)?;

    //3. Validar volumen mínimo
    if sessions.len() < min_sessions {
        tracing::error!(
            count = sessions.len(),
            threashold = min_sessions,
            "Scraper devolvió menos sesiones de las esperadas, abortando sincronización"
        );
        return Ok(SyncResult {
            aborted: true,
            abort_reason: Some(format!(
                "Scraper devolvió solo {} sesiones, menos que el mínimo esperado de {min_sessions}",
                sessions.len(),
            )),
            ..Default::default()
        });
    }

    //4. Obtener cambios aprobados, eliminar todas las sesiones e insertar las del scraper
    let approved_changes = repo.get_approved_changes().await?;
    tracing::info!(
        count = approved_changes.len(),
        "Cambios aprobados obtenidos"
    );

    repo.upsert_subject_groups_batch(&sessions).await?;

    let deleted = repo.delete_all_sessions().await?;
    tracing::info!(deleted, "Sesiones antiguas borradas");

    let sessions_inserted = repo.insert_sessions_batch(&sessions).await?;
    tracing::info!(
        inserted = sessions_inserted,
        "Sesiones del scraper insertadas"
    );

    //5. Aplicar cambios aprobados
    let mut result = SyncResult {
        sessions_inserted,
        ..Default::default()
    };

    for change in &approved_changes {
        match change.change_type {
            ChangeType::Create => apply_create(repo, change, &mut result).await?,
            ChangeType::Modify => {
                apply_modify(repo, notifications_repo, email_queue, change, &mut result).await?
            }
            ChangeType::Delete => {
                apply_delete(repo, notifications_repo, email_queue, change, &mut result).await?
            }
        }
    }

    //6. Archivar overrides caducados, rejected y pending huérfanos
    let overrides_expired = repo.archive_orphan_approved_changes().await?;
    result.overrides_expired = overrides_expired;
    tracing::info!(overrides_expired, "Cambios aprobados huérfanos archivados");

    let rejected_archived = repo.archive_rejected_changes().await?;
    result.rejected_archived = rejected_archived;
    tracing::info!(rejected_archived, "Cambios rechazados archivados");

    let pending_rejected = repo.reject_and_archive_orphan_pending().await?;
    result.pending_rejected = pending_rejected;
    tracing::info!(pending_rejected, "Cambios pendientes huérfanos rechazados");

    tracing::info!(
        sessions_inserted = result.sessions_inserted,
        sessions_from_changes = result.sessions_from_changes,
        changes_applied = result.changes_applied,
        changes_ignored = result.changes_ignored,
        overrides_expired = result.overrides_expired,
        pending_rejected = result.pending_rejected,
        rejected_archived = result.rejected_archived,
        "Sincronización completada"
    );

    Ok(result)
}

async fn apply_create(
    repo: &dyn ScraperRepository,
    change: &ApprovedChange,
    result: &mut SyncResult,
) -> Result<(), AppError> {
    let (subject, grp) = match (&change.subject, &change.grp) {
        (Some(s), Some(g)) => (s.as_str(), g.as_str()),
        _ => {
            tracing::warn!(change_id = %change.id, "Create sin subject/grp, ignorando");
            result.changes_ignored += 1;
            return Ok(());
        }
    };

    let starts_at = match change.new_starts_at {
        Some(t) => t,
        None => {
            tracing::warn!(change_id = %change.id, "Create sin new_starts_at, ignorando");
            result.changes_ignored += 1;
            return Ok(());
        }
    };

    let duration_min = match change.new_duration {
        Some(d) => d,
        None => {
            tracing::warn!(change_id = %change.id, "Create sin new_duration, ignorando");
            result.changes_ignored += 1;
            return Ok(());
        }
    };

    repo.insert_session_from_change(
        subject,
        grp,
        starts_at,
        duration_min,
        change.new_classroom.as_deref(),
        change.proposed_by,
    )
    .await?;

    result.sessions_from_changes += 1;
    Ok(())
}

async fn apply_modify(
    repo: &dyn ScraperRepository,
    notifications_repo: &dyn NotificationRepository,
    email_queue: &EmailQueue,
    change: &super::models::ApprovedChange,
    result: &mut SyncResult,
) -> Result<(), AppError> {
    let (subject, grp, prev_starts_at) =
        match (&change.subject, &change.grp, &change.prev_starts_at) {
            (Some(s), Some(g), Some(t)) => (s.as_str(), g.as_str(), *t),
            _ => {
                tracing::warn!(change_id = %change.id, "Modify sin prev_* completos — ignorado");
                result.changes_ignored += 1;
                return Ok(());
            }
        };

    let session_id = repo
        .find_session_by_natural_key(subject, grp, prev_starts_at)
        .await?;

    match session_id {
        Some(id) => {
            repo.apply_modify(
                id,
                change.new_starts_at,
                change.new_duration,
                change.new_classroom.as_deref(),
            )
            .await?;
            result.changes_applied += 1;
            tracing::debug!(
                change_id = %change.id,
                session_id = %id,
                "Modify aplicado"
            );
        }
        None => {
            // La sesión ya no existe en la fuente — ignorar
            tracing::debug!(
                change_id = %change.id,
                subject, grp,
                prev_starts_at = %prev_starts_at,
                "Modify ignorado — sesión de referencia no existe en BBDD"
            );
            result.changes_ignored += 1;

            notifications_service::notify_scraper_conflict(
                notifications_repo,
                email_queue,
                ScraperConflictInfo {
                    subject: subject.to_string(),
                    grp: grp.to_string(),
                    prev_starts_at,
                    reason: "El scraper ya no reporta esta sesión, pero había una propuesta de modificación aprobada pendiente de aplicar sobre ella.".to_string(),
                },
            )
            .await;
        }
    }

    Ok(())
}

async fn apply_delete(
    repo: &dyn ScraperRepository,
    notifications_repo: &dyn NotificationRepository,
    email_queue: &EmailQueue,
    change: &super::models::ApprovedChange,
    result: &mut SyncResult,
) -> Result<(), AppError> {
    let (subject, grp, prev_starts_at) =
        match (&change.subject, &change.grp, &change.prev_starts_at) {
            (Some(s), Some(g), Some(t)) => (s.as_str(), g.as_str(), *t),
            _ => {
                tracing::warn!(change_id = %change.id, "Delete sin prev_* completos — ignorado");
                result.changes_ignored += 1;
                return Ok(());
            }
        };

    let session_id = repo
        .find_session_by_natural_key(subject, grp, prev_starts_at)
        .await?;

    match session_id {
        Some(id) => {
            repo.delete_session_by_id(id).await?;
            result.changes_applied += 1;
            tracing::debug!(
                change_id = %change.id,
                session_id = %id,
                "Delete aplicado"
            );
        }
        None => {
            tracing::debug!(
                change_id = %change.id,
                subject, grp,
                prev_starts_at = %prev_starts_at,
                "Delete ignorado — sesión de referencia no existe en BBDD"
            );
            result.changes_ignored += 1;

            notifications_service::notify_scraper_conflict(
                notifications_repo,
                email_queue,
                ScraperConflictInfo {
                    subject: subject.to_string(),
                    grp: grp.to_string(),
                    prev_starts_at,
                    reason: "El scraper ya no reporta esta sesión, pero había una propuesta de eliminación aprobada pendiente de aplicar sobre ella.".to_string(),
                },
            )
            .await;
        }
    }

    Ok(())
}
