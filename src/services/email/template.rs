pub fn verification_email(verification_url: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Verifica tu cuenta – HorariosHub</title>
</head>
<body style="margin:0;padding:0;background-color:#f1f5f9;font-family:'Segoe UI',Helvetica,Arial,sans-serif;">
  <table width="100%" cellpadding="0" cellspacing="0" border="0" style="background-color:#f1f5f9;padding:40px 16px;">
    <tr>
      <td align="center">
        <table width="600" cellpadding="0" cellspacing="0" border="0" style="max-width:600px;width:100%;">

          <!-- Header -->
          <tr>
            <td style="background-color:#0f172a;border-radius:12px 12px 0 0;padding:32px 40px;text-align:center;">
              <span style="font-size:22px;font-weight:700;color:#ffffff;letter-spacing:-0.5px;">HorariosHub</span>
            </td>
          </tr>

          <!-- Body -->
          <tr>
            <td style="background-color:#ffffff;padding:48px 40px;border-left:1px solid #e2e8f0;border-right:1px solid #e2e8f0;">
              <p style="margin:0 0 8px;font-size:13px;font-weight:600;color:#6366f1;text-transform:uppercase;letter-spacing:1px;">Verificación de cuenta</p>
              <h1 style="margin:0 0 20px;font-size:26px;font-weight:700;color:#0f172a;line-height:1.3;">Confirma tu dirección<br>de correo electrónico</h1>
              <p style="margin:0 0 32px;font-size:15px;color:#475569;line-height:1.7;">
                Gracias por registrarte en HorariosHub. Para activar tu cuenta y comenzar a gestionar tus horarios, haz clic en el botón a continuación.
              </p>
              <table cellpadding="0" cellspacing="0" border="0" style="margin:0 0 32px;">
                <tr>
                  <td style="border-radius:8px;background-color:#6366f1;">
                    <a href="{verification_url}" style="display:inline-block;padding:14px 32px;font-size:15px;font-weight:600;color:#ffffff;text-decoration:none;border-radius:8px;letter-spacing:0.2px;">Verificar mi cuenta</a>
                  </td>
                </tr>
              </table>
              <p style="margin:0 0 8px;font-size:13px;color:#94a3b8;">Si el botón no funciona, copia y pega este enlace en tu navegador:</p>
              <p style="margin:0 0 32px;font-size:13px;word-break:break-all;">
                <a href="{verification_url}" style="color:#6366f1;text-decoration:none;">{verification_url}</a>
              </p>
              <table width="100%" cellpadding="0" cellspacing="0" border="0">
                <tr>
                  <td style="border-top:1px solid #f1f5f9;padding-top:24px;">
                    <p style="margin:0;font-size:13px;color:#94a3b8;line-height:1.6;">
                      Este enlace expirará en <strong style="color:#64748b;">24 horas</strong>. Si no creaste una cuenta en HorariosHub, puedes ignorar este mensaje con total seguridad.
                    </p>
                  </td>
                </tr>
              </table>
            </td>
          </tr>

          <!-- Footer -->
          <tr>
            <td style="background-color:#f8fafc;border-radius:0 0 12px 12px;padding:24px 40px;border:1px solid #e2e8f0;border-top:none;text-align:center;">
              <p style="margin:0 0 4px;font-size:12px;color:#94a3b8;">&copy; 2026 HorariosHub. Todos los derechos reservados.</p>
              <p style="margin:0;font-size:12px;color:#cbd5e1;">Este correo fue enviado automáticamente, por favor no respondas a este mensaje.</p>
            </td>
          </tr>

        </table>
      </td>
    </tr>
  </table>
</body>
</html>"#
    )
}

pub fn password_reset_email(reset_url: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Restablece tu contraseña – HorariosHub</title>
</head>
<body style="margin:0;padding:0;background-color:#f1f5f9;font-family:'Segoe UI',Helvetica,Arial,sans-serif;">
  <table width="100%" cellpadding="0" cellspacing="0" border="0" style="background-color:#f1f5f9;padding:40px 16px;">
    <tr>
      <td align="center">
        <table width="600" cellpadding="0" cellspacing="0" border="0" style="max-width:600px;width:100%;">

          <!-- Header -->
          <tr>
            <td style="background-color:#0f172a;border-radius:12px 12px 0 0;padding:32px 40px;text-align:center;">
              <span style="font-size:22px;font-weight:700;color:#ffffff;letter-spacing:-0.5px;">HorariosHub</span>
            </td>
          </tr>

          <!-- Alert banner -->
          <tr>
            <td style="background-color:#fef3c7;border-left:1px solid #e2e8f0;border-right:1px solid #e2e8f0;padding:12px 40px;">
              <p style="margin:0;font-size:13px;color:#92400e;text-align:center;">
                &#x26A0;&#xFE0F;&nbsp; Si no solicitaste este cambio, ignora este correo. Tu contraseña no será modificada.
              </p>
            </td>
          </tr>

          <!-- Body -->
          <tr>
            <td style="background-color:#ffffff;padding:48px 40px;border-left:1px solid #e2e8f0;border-right:1px solid #e2e8f0;">
              <p style="margin:0 0 8px;font-size:13px;font-weight:600;color:#f59e0b;text-transform:uppercase;letter-spacing:1px;">Seguridad de cuenta</p>
              <h1 style="margin:0 0 20px;font-size:26px;font-weight:700;color:#0f172a;line-height:1.3;">Restablece tu<br>contraseña</h1>
              <p style="margin:0 0 32px;font-size:15px;color:#475569;line-height:1.7;">
                Recibimos una solicitud para restablecer la contraseña asociada a tu cuenta de HorariosHub. Haz clic en el botón a continuación para crear una nueva contraseña.
              </p>
              <table cellpadding="0" cellspacing="0" border="0" style="margin:0 0 32px;">
                <tr>
                  <td style="border-radius:8px;background-color:#0f172a;">
                    <a href="{reset_url}" style="display:inline-block;padding:14px 32px;font-size:15px;font-weight:600;color:#ffffff;text-decoration:none;border-radius:8px;letter-spacing:0.2px;">Restablecer contraseña</a>
                  </td>
                </tr>
              </table>
              <p style="margin:0 0 8px;font-size:13px;color:#94a3b8;">Si el botón no funciona, copia y pega este enlace en tu navegador:</p>
              <p style="margin:0 0 32px;font-size:13px;word-break:break-all;">
                <a href="{reset_url}" style="color:#6366f1;text-decoration:none;">{reset_url}</a>
              </p>
              <table width="100%" cellpadding="0" cellspacing="0" border="0">
                <tr>
                  <td style="border-top:1px solid #f1f5f9;padding-top:24px;">
                    <p style="margin:0;font-size:13px;color:#94a3b8;line-height:1.6;">
                      Este enlace expirará en <strong style="color:#64748b;">1 hora</strong> por razones de seguridad. Si necesitas un nuevo enlace, vuelve a iniciar el proceso de recuperación desde la aplicación.
                    </p>
                  </td>
                </tr>
              </table>
            </td>
          </tr>

          <!-- Footer -->
          <tr>
            <td style="background-color:#f8fafc;border-radius:0 0 12px 12px;padding:24px 40px;border:1px solid #e2e8f0;border-top:none;text-align:center;">
              <p style="margin:0 0 4px;font-size:12px;color:#94a3b8;">&copy; 2026 HorariosHub. Todos los derechos reservados.</p>
              <p style="margin:0;font-size:12px;color:#cbd5e1;">Este correo fue enviado automáticamente, por favor no respondas a este mensaje.</p>
            </td>
          </tr>

        </table>
      </td>
    </tr>
  </table>
</body>
</html>"#
    )
}

pub fn notification_email(subject: &str, message: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{subject} – HorariosHub</title>
</head>
<body style="margin:0;padding:0;background-color:#f1f5f9;font-family:'Segoe UI',Helvetica,Arial,sans-serif;">
  <table width="100%" cellpadding="0" cellspacing="0" border="0" style="background-color:#f1f5f9;padding:40px 16px;">
    <tr>
      <td align="center">
        <table width="600" cellpadding="0" cellspacing="0" border="0" style="max-width:600px;width:100%;">

          <!-- Header -->
          <tr>
            <td style="background-color:#0f172a;border-radius:12px 12px 0 0;padding:32px 40px;text-align:center;">
              <span style="font-size:22px;font-weight:700;color:#ffffff;letter-spacing:-0.5px;">HorariosHub</span>
            </td>
          </tr>

          <!-- Body -->
          <tr>
            <td style="background-color:#ffffff;padding:48px 40px;border-left:1px solid #e2e8f0;border-right:1px solid #e2e8f0;">
              <p style="margin:0 0 8px;font-size:13px;font-weight:600;color:#6366f1;text-transform:uppercase;letter-spacing:1px;">Notificación</p>
              <h1 style="margin:0 0 20px;font-size:26px;font-weight:700;color:#0f172a;line-height:1.3;">{subject}</h1>
              <div style="font-size:15px;color:#475569;line-height:1.7;">
                <p style="margin:0;">{message}</p>
              </div>
            </td>
          </tr>

          <!-- Footer -->
          <tr>
            <td style="background-color:#f8fafc;border-radius:0 0 12px 12px;padding:24px 40px;border:1px solid #e2e8f0;border-top:none;text-align:center;">
              <p style="margin:0 0 4px;font-size:12px;color:#94a3b8;">&copy; 2026 HorariosHub. Todos los derechos reservados.</p>
              <p style="margin:0;font-size:12px;color:#cbd5e1;">Este correo fue enviado automáticamente, por favor no respondas a este mensaje.</p>
            </td>
          </tr>

        </table>
      </td>
    </tr>
  </table>
</body>
</html>"#
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
        assert!(html.contains("<html"));
        assert!(html.contains("</html>"));
        assert!(html.contains("<body"));
    }

    #[test]
    fn password_reset_email_es_html_valido() {
        let html = password_reset_email("https://example.com/reset");
        assert!(html.contains("<html"));
        assert!(html.contains("</html>"));
        assert!(html.contains("<body"));
    }
}
