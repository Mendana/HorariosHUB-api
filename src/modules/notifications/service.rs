use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    modules::{
        notifications::{
            models::{
                NewNotification, NotificationType, NotifyRecipient, ScraperConflictInfo,
                SessionChangeType,
            },
            repository::NotificationRepository,
        },
        users::models::UserRole,
    },
    services::email::service::EmailService,
};


pub struct EmailJob {
    pub to: String,
    pub subject: String,
    pub message: String,
}

#[derive(Clone)]
pub struct EmailQueue(mpsc::Sender<EmailJob>);

impl EmailQueue {
    pub fn new(capacity: usize) -> (Self, mpsc::Receiver<EmailJob>) {
        let (tx, rx) = mpsc::channel(capacity);
        (Self(tx), rx)
    }

    pub fn enqueue(&self, job: EmailJob) {
        if let Err(e) = self.0.try_send(job) {
            tracing::error!(?e, "No se pudo encolar el email de notificación");
        }
    }
}

pub async fn run_email_worker(mut rx: mpsc::Receiver<EmailJob>, email: Arc<dyn EmailService>) {
    while let Some(job) = rx.recv().await {
        if let Err(e) = email
            .send_notification_email(&job.to, &job.subject, &job.message)
            .await
        {
            tracing::error!(?e, to = %job.to, "Fallo al enviar email de notificación");
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

struct NotificationContent<'a> {
    r#type: NotificationType,
    title: &'a str,
    body: &'a str,
    session_id: Option<Uuid>,
    proposal_id: Option<Uuid>,
}

async fn persist_and_queue_emails(
    repo: &dyn NotificationRepository,
    email_queue: &EmailQueue,
    recipients: &[NotifyRecipient],
    content: NotificationContent<'_>,
) {
    let new_notifications: Vec<NewNotification> = recipients
        .iter()
        .filter(|r| r.notify_in_app)
        .map(|r| NewNotification {
            user_id: r.user_id,
            r#type: content.r#type.clone(),
            title: content.title.to_string(),
            body: content.body.to_string(),
            session_id: content.session_id,
            proposal_id: content.proposal_id,
        })
        .collect();

    if !new_notifications.is_empty() {
        repo.insert_many(&new_notifications).await.unwrap_or_else(|e| {
            tracing::error!(?e, r#type = ?content.r#type, "No se pudieron insertar las notificaciones in-app");
        });
    }

    for r in recipients.iter().filter(|r| r.notify_email) {
        email_queue.enqueue(EmailJob {
            to: r.email.clone(),
            subject: content.title.to_string(),
            message: content.body.to_string(),
        });
    }
}

pub async fn notify_session_modified(
    repo: &dyn NotificationRepository,
    email_queue: &EmailQueue,
    session_id: Uuid,
    subject: &str,
    grp: &str,
    change_type: SessionChangeType,
) {
    let recipients = match repo.find_subscribers_by_session(session_id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(?e, %session_id, "No se pudieron obtener los suscriptores de la sesión");
            return;
        }
    };
    if recipients.is_empty() {
        return;
    }

    let (notif_type, title, body) = match change_type {
        SessionChangeType::Modified => (
            NotificationType::SessionModified,
            format!("Clase modificada: {subject} ({grp})"),
            format!("La clase de {subject} ({grp}) ha sido modificada."),
        ),
        SessionChangeType::Deleted => (
            NotificationType::SessionDeleted,
            format!("Clase eliminada: {subject} ({grp})"),
            format!("La clase de {subject} ({grp}) ha sido eliminada."),
        ),
    };

    persist_and_queue_emails(
        repo,
        email_queue,
        &recipients,
        NotificationContent {
            r#type: notif_type,
            title: &title,
            body: &body,
            session_id: Some(session_id),
            proposal_id: None,
        },
    )
    .await;
}

pub async fn notify_exam_added(
    repo: &dyn NotificationRepository,
    email_queue: &EmailQueue,
    session_id: Uuid,
    subject: &str,
    grp: &str,
    starts_at: DateTime<Utc>,
) {
    let recipients = match repo.find_subscribers_by_session(session_id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(?e, %session_id, "No se pudieron obtener los suscriptores para el examen");
            return;
        }
    };
    if recipients.is_empty() {
        return;
    }

    let title = format!("Nuevo examen: {subject} ({grp})");
    let body = format!("Se ha añadido un examen de {subject} ({grp}) el {starts_at}.");

    persist_and_queue_emails(
        repo,
        email_queue,
        &recipients,
        NotificationContent {
            r#type: NotificationType::ExamAdded,
            title: &title,
            body: &body,
            session_id: Some(session_id),
            proposal_id: None,
        },
    )
    .await;
}

pub async fn notify_proposal_status_changed(
    repo: &dyn NotificationRepository,
    email_queue: &EmailQueue,
    proposal_id: Uuid,
    proposed_by: Uuid,
    approved: bool,
) {
    let recipient = match repo.find_user_by_id(proposed_by).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            tracing::warn!(%proposed_by, "Autor de la propuesta no encontrado, no se notifica");
            return;
        }
        Err(e) => {
            tracing::error!(?e, %proposed_by, "No se pudo obtener el autor de la propuesta");
            return;
        }
    };

    let (notif_type, title, body) = if approved {
        (
            NotificationType::ProposalApproved,
            "Propuesta aprobada".to_string(),
            "Tu propuesta de cambio ha sido aprobada.".to_string(),
        )
    } else {
        (
            NotificationType::ProposalRejected,
            "Propuesta rechazada".to_string(),
            "Tu propuesta de cambio ha sido rechazada.".to_string(),
        )
    };

    persist_and_queue_emails(
        repo,
        email_queue,
        std::slice::from_ref(&recipient),
        NotificationContent {
            r#type: notif_type,
            title: &title,
            body: &body,
            session_id: None,
            proposal_id: Some(proposal_id),
        },
    )
    .await;
}

pub async fn notify_proposal_created(
    repo: &dyn NotificationRepository,
    email_queue: &EmailQueue,
    proposal_id: Uuid,
) {
    let recipients = match repo
        .find_users_by_roles(&[UserRole::Professor, UserRole::Admin])
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(?e, %proposal_id, "No se pudieron obtener los revisores");
            return;
        }
    };
    if recipients.is_empty() {
        return;
    }

    persist_and_queue_emails(
        repo,
        email_queue,
        &recipients,
        NotificationContent {
            r#type: NotificationType::ProposalCreated,
            title: "Nueva propuesta pendiente de revisión",
            body: "Hay una nueva propuesta de cambio pendiente de revisión.",
            session_id: None,
            proposal_id: Some(proposal_id),
        },
    )
    .await;
}

pub async fn notify_scraper_conflict(
    repo: &dyn NotificationRepository,
    email_queue: &EmailQueue,
    conflict: ScraperConflictInfo,
) {
    let recipients = match repo.find_users_by_roles(&[UserRole::Admin]).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(?e, "No se pudieron obtener los administradores");
            return;
        }
    };
    if recipients.is_empty() {
        return;
    }

    let title = format!("Conflicto detectado: {} ({})", conflict.subject, conflict.grp);

    persist_and_queue_emails(
        repo,
        email_queue,
        &recipients,
        NotificationContent {
            r#type: NotificationType::ScrapperConflict,
            title: &title,
            body: &conflict.reason,
            session_id: None,
            proposal_id: None,
        },
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::AppError;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockNotificationRepository {
        subscribers: Vec<NotifyRecipient>,
        users_by_role: Vec<NotifyRecipient>,
        user_by_id: Option<NotifyRecipient>,
        fail_lookup: bool,
        fail_insert: bool,
        inserted: Mutex<Vec<NewNotification>>,
    }

    #[async_trait::async_trait]
    impl NotificationRepository for MockNotificationRepository {
        async fn find_subscribers_by_session(
            &self,
            _session_id: Uuid,
        ) -> Result<Vec<NotifyRecipient>, AppError> {
            if self.fail_lookup {
                return Err(AppError::Internal(anyhow::anyhow!("db error")));
            }
            Ok(self.subscribers.clone())
        }

        async fn find_users_by_roles(
            &self,
            _roles: &[UserRole],
        ) -> Result<Vec<NotifyRecipient>, AppError> {
            if self.fail_lookup {
                return Err(AppError::Internal(anyhow::anyhow!("db error")));
            }
            Ok(self.users_by_role.clone())
        }

        async fn insert_many(&self, notifications: &[NewNotification]) -> Result<(), AppError> {
            if self.fail_insert {
                return Err(AppError::Internal(anyhow::anyhow!("insert failed")));
            }
            self.inserted.lock().unwrap().extend_from_slice(notifications);
            Ok(())
        }

        async fn find_user_by_id(&self, _user_id: Uuid) -> Result<Option<NotifyRecipient>, AppError> {
            if self.fail_lookup {
                return Err(AppError::Internal(anyhow::anyhow!("db error")));
            }
            Ok(self.user_by_id.clone())
        }
    }

    fn recipient(email: &str, notify_in_app: bool, notify_email: bool) -> NotifyRecipient {
        NotifyRecipient {
            user_id: Uuid::new_v4(),
            email: email.to_string(),
            notify_in_app,
            notify_email,
        }
    }

    #[tokio::test]
    async fn notify_session_modified_respeta_preferencias_de_cada_suscriptor() {
        let in_app_only = recipient("in-app@test.com", true, false);
        let email_only = recipient("email@test.com", false, true);
        let silencioso = recipient("silencioso@test.com", false, false);

        let repo = MockNotificationRepository {
            subscribers: vec![in_app_only.clone(), email_only.clone(), silencioso],
            ..Default::default()
        };
        let (queue, mut rx) = EmailQueue::new(10);

        notify_session_modified(
            &repo,
            &queue,
            Uuid::new_v4(),
            "ALG",
            "Teoría",
            SessionChangeType::Modified,
        )
        .await;

        let inserted = repo.inserted.lock().unwrap();
        assert_eq!(inserted.len(), 1);
        assert_eq!(inserted[0].user_id, in_app_only.user_id);
        assert!(matches!(inserted[0].r#type, NotificationType::SessionModified));
        drop(inserted);

        let job = rx.try_recv().expect("debe haberse encolado un email");
        assert_eq!(job.to, "email@test.com");
        assert!(rx.try_recv().is_err(), "solo debe encolarse un email");
    }

    #[tokio::test]
    async fn notify_session_modified_deleted_usa_el_tipo_correcto() {
        let sub = recipient("stu@test.com", true, false);
        let repo = MockNotificationRepository {
            subscribers: vec![sub],
            ..Default::default()
        };
        let (queue, _rx) = EmailQueue::new(10);

        notify_session_modified(
            &repo,
            &queue,
            Uuid::new_v4(),
            "ALG",
            "Teoría",
            SessionChangeType::Deleted,
        )
        .await;

        let inserted = repo.inserted.lock().unwrap();
        assert!(matches!(inserted[0].r#type, NotificationType::SessionDeleted));
    }

    #[tokio::test]
    async fn notify_session_modified_sin_suscriptores_no_hace_nada() {
        let repo = MockNotificationRepository::default();
        let (queue, mut rx) = EmailQueue::new(10);

        notify_session_modified(
            &repo,
            &queue,
            Uuid::new_v4(),
            "ALG",
            "Teoría",
            SessionChangeType::Modified,
        )
        .await;

        assert!(repo.inserted.lock().unwrap().is_empty());
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn notify_session_modified_error_del_repo_no_hace_panic() {
        let repo = MockNotificationRepository {
            fail_lookup: true,
            ..Default::default()
        };
        let (queue, mut rx) = EmailQueue::new(10);

        // No debe entrar en pánico ni propagar el error: la notificación es best-effort.
        notify_session_modified(
            &repo,
            &queue,
            Uuid::new_v4(),
            "ALG",
            "Teoría",
            SessionChangeType::Modified,
        )
        .await;

        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn notify_exam_added_inserta_tipo_exam_added() {
        let sub = recipient("stu@test.com", true, true);
        let repo = MockNotificationRepository {
            subscribers: vec![sub],
            ..Default::default()
        };
        let (queue, mut rx) = EmailQueue::new(10);

        notify_exam_added(&repo, &queue, Uuid::new_v4(), "MAT", "Examen", Utc::now()).await;

        let inserted = repo.inserted.lock().unwrap();
        assert_eq!(inserted.len(), 1);
        assert!(matches!(inserted[0].r#type, NotificationType::ExamAdded));
        drop(inserted);
        assert!(rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn notify_proposal_status_changed_aprobada_notifica_al_autor() {
        let author = recipient("author@test.com", true, true);
        let author_id = author.user_id;
        let repo = MockNotificationRepository {
            user_by_id: Some(author),
            ..Default::default()
        };
        let (queue, mut rx) = EmailQueue::new(10);
        let proposal_id = Uuid::new_v4();

        notify_proposal_status_changed(&repo, &queue, proposal_id, author_id, true).await;

        let inserted = repo.inserted.lock().unwrap();
        assert_eq!(inserted.len(), 1);
        assert_eq!(inserted[0].user_id, author_id);
        assert_eq!(inserted[0].proposal_id, Some(proposal_id));
        assert!(matches!(inserted[0].r#type, NotificationType::ProposalApproved));
        drop(inserted);
        assert!(rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn notify_proposal_status_changed_rechazada_usa_el_tipo_correcto() {
        let author = recipient("author@test.com", true, false);
        let author_id = author.user_id;
        let repo = MockNotificationRepository {
            user_by_id: Some(author),
            ..Default::default()
        };
        let (queue, _rx) = EmailQueue::new(10);

        notify_proposal_status_changed(&repo, &queue, Uuid::new_v4(), author_id, false).await;

        let inserted = repo.inserted.lock().unwrap();
        assert!(matches!(inserted[0].r#type, NotificationType::ProposalRejected));
    }

    #[tokio::test]
    async fn notify_proposal_status_changed_autor_no_encontrado_no_hace_nada() {
        let repo = MockNotificationRepository {
            user_by_id: None,
            ..Default::default()
        };
        let (queue, mut rx) = EmailQueue::new(10);

        notify_proposal_status_changed(&repo, &queue, Uuid::new_v4(), Uuid::new_v4(), true).await;

        assert!(repo.inserted.lock().unwrap().is_empty());
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn notify_proposal_created_notifica_a_todos_los_revisores() {
        let prof = recipient("prof@test.com", true, false);
        let admin = recipient("admin@test.com", true, true);
        let repo = MockNotificationRepository {
            users_by_role: vec![prof, admin],
            ..Default::default()
        };
        let (queue, mut rx) = EmailQueue::new(10);

        notify_proposal_created(&repo, &queue, Uuid::new_v4()).await;

        let inserted = repo.inserted.lock().unwrap();
        assert_eq!(inserted.len(), 2);
        assert!(
            inserted
                .iter()
                .all(|n| matches!(n.r#type, NotificationType::ProposalCreated))
        );
        drop(inserted);

        // Solo el admin tiene notify_email = true
        let job = rx.try_recv().expect("debe haberse encolado un email");
        assert_eq!(job.to, "admin@test.com");
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn notify_scraper_conflict_notifica_solo_a_admins() {
        let admin = recipient("admin@test.com", true, true);
        let repo = MockNotificationRepository {
            users_by_role: vec![admin],
            ..Default::default()
        };
        let (queue, mut rx) = EmailQueue::new(10);

        notify_scraper_conflict(
            &repo,
            &queue,
            ScraperConflictInfo {
                subject: "ALG".to_string(),
                grp: "Teoría".to_string(),
                prev_starts_at: Utc::now(),
                reason: "conflicto de prueba".to_string(),
            },
        )
        .await;

        let inserted = repo.inserted.lock().unwrap();
        assert_eq!(inserted.len(), 1);
        assert!(matches!(inserted[0].r#type, NotificationType::ScrapperConflict));
        assert!(inserted[0].title.contains("ALG"));
        drop(inserted);
        assert!(rx.try_recv().is_ok());
    }
}
