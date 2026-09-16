use crate::{
    errors::AppError,
    modules::feedback::models::{SubmitFeedbackRequest, SubmitFeedbackResponse},
    services::email::service::EmailService,
};

const DEFAULT_SUBJECT: &str = "Nuevo mensaje de contacto";

/// Envía el mensaje del formulario de contacto a todos los `recipients`.
///
/// Se considera éxito si al menos un destinatario recibió el email: la lista
/// de recipients es fija (configurada por un admin), así que un fallo puntual
/// de un buzón no debería hacer fallar la petición del usuario que envía el
/// formulario.
#[tracing::instrument(skip(email_service, recipients, payload), fields(from_email = %payload.email))]
pub async fn submit_feedback(
    email_service: &dyn EmailService,
    recipients: &[String],
    payload: SubmitFeedbackRequest,
) -> Result<SubmitFeedbackResponse, AppError> {
    let subject = payload
        .subject
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_SUBJECT);

    let mut last_error = None;
    let mut any_sent = false;

    for to in recipients {
        match email_service
            .send_feedback_email(to, &payload.name, &payload.email, subject, &payload.body)
            .await
        {
            Ok(()) => any_sent = true,
            Err(e) => {
                tracing::error!(error = ?e, to = %to, "Fallo al enviar el email del formulario de contacto");
                last_error = Some(e);
            }
        }
    }

    if any_sent {
        tracing::info!("Mensaje del formulario de contacto enviado");
        Ok(SubmitFeedbackResponse {
            message: "Mensaje enviado correctamente".to_string(),
        })
    } else {
        Err(last_error
            .unwrap_or_else(|| AppError::Internal(anyhow::anyhow!("No se pudo enviar el mensaje"))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Debug, PartialEq)]
    struct RecordedSend {
        to: String,
        name: String,
        from_email: String,
        subject: String,
        body: String,
    }

    /// Test double que registra cada llamada a `send_feedback_email` y falla
    /// para las direcciones listadas en `fail_for`.
    #[derive(Default)]
    struct RecordingEmailService {
        fail_for: Vec<String>,
        sent: Mutex<Vec<RecordedSend>>,
    }

    #[async_trait]
    impl EmailService for RecordingEmailService {
        async fn send_verification_email(&self, _to: &str, _token: &str) -> Result<(), AppError> {
            Ok(())
        }
        async fn send_password_reset_email(
            &self,
            _to: &str,
            _token: &str,
        ) -> Result<(), AppError> {
            Ok(())
        }
        async fn send_notification_email(
            &self,
            _to: &str,
            _subject: &str,
            _message: &str,
        ) -> Result<(), AppError> {
            Ok(())
        }
        async fn send_feedback_email(
            &self,
            to: &str,
            name: &str,
            from_email: &str,
            subject: &str,
            body: &str,
        ) -> Result<(), AppError> {
            self.sent.lock().unwrap().push(RecordedSend {
                to: to.to_string(),
                name: name.to_string(),
                from_email: from_email.to_string(),
                subject: subject.to_string(),
                body: body.to_string(),
            });

            if self.fail_for.contains(&to.to_string()) {
                Err(AppError::Internal(anyhow::anyhow!("smtp caido")))
            } else {
                Ok(())
            }
        }
    }

    fn payload(subject: Option<&str>) -> SubmitFeedbackRequest {
        SubmitFeedbackRequest {
            name: "Ana".to_string(),
            email: "ana@uniovi.es".to_string(),
            subject: subject.map(str::to_string),
            body: "El horario no se ve bien en móvil.".to_string(),
        }
    }

    #[tokio::test]
    async fn envia_a_todos_los_destinatarios_configurados() {
        let email = RecordingEmailService::default();
        let recipients = vec!["admin1@uniovi.es".to_string(), "admin2@uniovi.es".to_string()];

        let response = submit_feedback(&email, &recipients, payload(Some("Sugerencia")))
            .await
            .unwrap();

        assert_eq!(response.message, "Mensaje enviado correctamente");
        let sent = email.sent.lock().unwrap();
        assert_eq!(sent.len(), 2);
        assert!(sent.iter().any(|s| s.to == "admin1@uniovi.es"));
        assert!(sent.iter().any(|s| s.to == "admin2@uniovi.es"));
        assert!(sent.iter().all(|s| s.subject == "Sugerencia"));
        assert!(sent.iter().all(|s| s.name == "Ana"));
        assert!(sent.iter().all(|s| s.from_email == "ana@uniovi.es"));
    }

    #[tokio::test]
    async fn usa_asunto_por_defecto_si_no_se_envia_ninguno() {
        let email = RecordingEmailService::default();
        let recipients = vec!["admin@uniovi.es".to_string()];

        submit_feedback(&email, &recipients, payload(None))
            .await
            .unwrap();

        let sent = email.sent.lock().unwrap();
        assert_eq!(sent[0].subject, DEFAULT_SUBJECT);
    }

    #[tokio::test]
    async fn usa_asunto_por_defecto_si_el_enviado_esta_vacio() {
        let email = RecordingEmailService::default();
        let recipients = vec!["admin@uniovi.es".to_string()];

        submit_feedback(&email, &recipients, payload(Some("   ")))
            .await
            .unwrap();

        let sent = email.sent.lock().unwrap();
        assert_eq!(sent[0].subject, DEFAULT_SUBJECT);
    }

    #[tokio::test]
    async fn exito_parcial_sigue_siendo_ok() {
        let email = RecordingEmailService {
            fail_for: vec!["falla@uniovi.es".to_string()],
            ..Default::default()
        };
        let recipients = vec!["falla@uniovi.es".to_string(), "ok@uniovi.es".to_string()];

        let response = submit_feedback(&email, &recipients, payload(None)).await;

        assert!(response.is_ok());
        assert_eq!(email.sent.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn devuelve_error_si_fallan_todos_los_destinatarios() {
        let email = RecordingEmailService {
            fail_for: vec!["admin@uniovi.es".to_string()],
            ..Default::default()
        };
        let recipients = vec!["admin@uniovi.es".to_string()];

        let response = submit_feedback(&email, &recipients, payload(None)).await;

        assert!(response.is_err());
    }
}
