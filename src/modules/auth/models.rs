use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;
use validator::Validate;

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
    pub id: Uuid,
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

/// Información pública del usuario que se devuelve en la respuesta de login
#[derive(Debug, Serialize)]
pub struct UserPublic {
    pub email: String,
    pub role: UserRole,
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

// --- Modelos de base de datos ---

/// Modelo de usuario para la base de datos
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub role: UserRole,
    pub verified: bool,
}

/// Modelo de verificación de email para la base de datos
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct VerificationToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Modelo de restablecimiento de contraseña para la base de datos
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PasswordResetToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Student,
    Professor,
    Admin,
}
