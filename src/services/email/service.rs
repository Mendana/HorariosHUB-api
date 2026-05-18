use async_trait::async_trait;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    address::AddressError,
    message::header::ContentType,
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
};

use crate::errors::AppError;
use crate::services::email::template;

#[async_trait]
pub trait EmailService: Send + Sync {
    async fn send_verification_email(&self, to: &str, token: &str) -> Result<(), AppError>;
    async fn send_password_reset_email(&self, to: &str, token: &str) -> Result<(), AppError>;
}

pub struct SmtpEmailService {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from_address: String,
    base_url: String,
}

pub struct MockEmailService;

#[async_trait]
impl EmailService for MockEmailService {
    async fn send_verification_email(&self, _to: &str, _token: &str) -> Result<(), AppError> {
        Ok(())
    }
    async fn send_password_reset_email(&self, _to: &str, _token: &str) -> Result<(), AppError> {
        Ok(())
    }
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

        let tls_params = TlsParameters::new_rustls(smtp_host.to_string())
            .map_err(|e| AppError::Internal(e.into()))?;

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(smtp_host)
            .port(smtp_port)
            .tls(Tls::Wrapper(tls_params))
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

#[async_trait]
impl EmailService for SmtpEmailService {
    async fn send_verification_email(&self, to: &str, token: &str) -> Result<(), AppError> {
        let url = format!("{}/auth/verify?token={}", self.base_url, token);
        let html = template::verification_email(&url);
        self.send_html(to, "Verifica tu cuenta en HorariosHub", &html)
            .await
    }

    async fn send_password_reset_email(&self, to: &str, token: &str) -> Result<(), AppError> {
        let url = format!("{}/auth/reset-password?token={}", self.base_url, token);
        let html = template::password_reset_email(&url);
        self.send_html(to, "Restablece tu contraseña en HorariosHub", &html)
            .await
    }
}
