pub fn verification_email(verification_url: &str) -> String {
    format!(
        r#"
        <html>
            <body>
                <h1>Verifica tu cuenta en HorariosHub</h1>
                <p>Haz clic en el siguiente enlace para verificar tu cuenta:</p>
                <a href="{verification_url}">Verificar mi cuenta</a>
                <p>El enlace expirará en 24 horas.</p>
            </body>
        </html>
        "#
    )
}

pub fn password_reset_email(reset_url: &str) -> String {
    format!(
        r#"
        <html>
            <body>
                <h1>Restablece tu contraseña en HorariosHub</h1>
                <p>Haz clic en el siguiente enlace para restablecer tu contraseña:</p>
                <a href="{reset_url}">Restablecer mi contraseña</a>
                <p>El enlace expirará en 1 hora.</p>
            </body>
        </html>
        "#
    )
}

pub fn notification_email(subject: &str, message: &str) -> String {
    format!(
        r#"
        <html>
            <body>
                <h1>{subject}</h1>
                <p>{message}</p>
            </body>
        </html>
        "#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_email_contiene_url() {
        let url = "https://app.horarioshub.com/auth/verify?token=abc123";
        let html = verification_email(url);
        assert!(html.contains(url));
        assert!(html.contains("<a href="));
    }

    #[test]
    fn password_reset_email_contiene_url() {
        let url = "https://app.horarioshub.com/auth/reset?token=xyz789";
        let html = password_reset_email(url);
        assert!(html.contains(url));
        assert!(html.contains("<a href="));
    }

    #[test]
    fn notification_email_contiene_subject_y_mensaje() {
        let subject = "Bienvenido a HorariosHub";
        let message = "Tu cuenta ha sido creada correctamente.";
        let html = notification_email(subject, message);
        assert!(html.contains(subject));
        assert!(html.contains(message));
    }

    #[test]
    fn verification_email_es_html_valido() {
        let html = verification_email("https://example.com/verify");
        assert!(html.contains("<html>"));
        assert!(html.contains("</html>"));
        assert!(html.contains("<body>"));
    }

    #[test]
    fn password_reset_email_es_html_valido() {
        let html = password_reset_email("https://example.com/reset");
        assert!(html.contains("<html>"));
        assert!(html.contains("</html>"));
        assert!(html.contains("<body>"));
    }
}
