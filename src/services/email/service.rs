use crate::{errors::AppError, services::email::template};
use async_trait::async_trait;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, address::AddressError,
    message::header::ContentType, transport::smtp::authentication::Credentials,
};

#[async_trait]
pub trait EmailService: Send + Sync {
    /// Envía un correo de verificación al usuario con el token proporcionado.
    ///
    /// # Argumentos
    /// - `to`: La dirección de correo electrónico del destinatario.
    /// - `token`: El token de verificación que se incluirá en el correo.
    async fn send_verification_email(&self, to: &str, token: &str) -> Result<(), AppError>;

    /// Envía un correo de restablecimiento de contraseña al usuario con el token proporcionado.
    ///
    /// # Argumentos
    /// - `to`: La dirección de correo electrónico del destinatario.
    /// - `token`: El token de restablecimiento de contraseña que se incluirá en el correo.
    async fn send_password_reset_email(&self, to: &str, token: &str) -> Result<(), AppError>;

    /// Envía una notificación genérica al usuario.
    ///
    /// # Argumentos
    /// - `to`: La dirección de correo electrónico del destinatario.
    /// - `subject`: El asunto del correo.
    /// - `body`: El cuerpo del correo.
    async fn send_notification(&self, to: &str, subject: &str, body: &str) -> Result<(), AppError>;
}

pub struct SmtpEmailService {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from_address: String,
    base_url: String,
}

impl SmtpEmailService {
    pub fn new(
        smtp_host: &str,
        smtp_port: u16,
        smtp_user: &str,
        smtp_password: &str,
        from_address: &str,
        base_url: &str,
    ) -> Result<Self, AppError> {
        let credentials = Credentials::new(smtp_user.to_string(), smtp_password.to_string());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_host)
            .map_err(|e| AppError::Internal(e.into()))?
            .port(smtp_port)
            .credentials(credentials)
            .build();

        Ok(Self {
            mailer,
            from_address: from_address.to_string(),
            base_url: base_url.to_string(),
        })
    }

    async fn send_html(&self, to: &str, subject: &str, html_body: &str) -> Result<(), AppError> {
        let email = Message::builder()
            .from(
                self.from_address
                    .parse()
                    .map_err(|e: AddressError| AppError::Internal(e.into()))?,
            )
            .to(to
                .parse()
                .map_err(|e: AddressError| AppError::Internal(e.into()))?)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html_body.to_string())
            .map_err(|e| AppError::Internal(e.into()))?;

        self.mailer
            .send(email)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl EmailService for SmtpEmailService {
    async fn send_verification_email(&self, to: &str, token: &str) -> Result<(), AppError> {
        let verification_link = format!("{}/verify?token={}", self.base_url, token);
        let subject = "Verificación de cuenta";
        let body = format!(
            "Hola,\n\nPor favor haz clic en el siguiente enlace para verificar tu cuenta:\n{}\n\nGracias.",
            verification_link
        );

        self.send_notification(to, subject, &body).await
    }

    async fn send_password_reset_email(&self, to: &str, token: &str) -> Result<(), AppError> {
        let url = format!("{}/auth/verify?token={token}", self.base_url);
        let body = template::verification_email(&url);
        self.send_html(to, "Verifica tu cuenta en HorariosHub", &body)
            .await
    }

    async fn send_notification(&self, to: &str, subject: &str, body: &str) -> Result<(), AppError> {
        let html = template::notification_email(subject, body);
        self.send_html(to, subject, &html).await
    }
}

/// Mock del servicio de email que no envía correos reales. Útil en tests y entornos sin SMTP.
pub struct MockEmailService;

#[async_trait]
impl EmailService for MockEmailService {
    async fn send_verification_email(&self, _to: &str, _token: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn send_password_reset_email(&self, _to: &str, _token: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn send_notification(
        &self,
        _to: &str,
        _subject: &str,
        _body: &str,
    ) -> Result<(), AppError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_send_verification_email_ok() {
        let service = MockEmailService;
        let result = service
            .send_verification_email("test@example.com", "token123")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn mock_send_password_reset_email_ok() {
        let service = MockEmailService;
        let result = service
            .send_password_reset_email("test@example.com", "reset_token")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn mock_send_notification_ok() {
        let service = MockEmailService;
        let result = service
            .send_notification("test@example.com", "Asunto de prueba", "Cuerpo del mensaje")
            .await;
        assert!(result.is_ok());
    }
}
