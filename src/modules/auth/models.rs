use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::modules::users::models::{UserPublic, UserRole};

/// Payload del enpoint `POST /auth/register`
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Email inválido"))]
    pub email: String,

    #[validate(length(min = 8, message = "La contraseña debe tener al menos 8 caracteres"))]
    pub password: String,
}

/// Respuesta del endpoint `POST /auth/register`
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub email: String,
    pub role: UserRole,
}

/// Payload del enpoint `POST /auth/login`
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Email inválido"))]
    pub email: String,

    #[validate(length(min = 1, message = "Contraseña requerida"))]
    pub password: String,
}

/// Respuesta del endpoint `POST /auth/login`
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: UserPublic,
}

/// Payload del endpoint `GET /auth/verify`
#[derive(Debug, Deserialize)]
pub struct VerifyEmailQuery {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyEmailResponse {
    pub message: String,
}

/// Payload del endpoint `POST /auth/reset-password`
#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordRequest {
    #[validate(length(min = 1, message = "Token requerido"))]
    pub token: String,

    #[validate(length(min = 8, message = "La contraseña debe tener al menos 8 caracteres"))]
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct ResetPasswordResponse {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub message: String,
}

/// Payload del endpoint `POST /auth/recover`
#[derive(Debug, Deserialize, Validate)]
pub struct RecoverPasswordRequest {
    #[validate(email(message = "Email inválido"))]
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct RecoverPasswordResponse {
    pub message: String,
}
