use serde::{Deserialize, Serialize};
use sqlx::prelude::Type;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct UserPublic {
    pub email: String,
    pub role: UserRole,
}

#[derive(Debug, Serialize)]
pub struct UsersListResponse {
    pub users: Vec<UserPublic>,
}

/// Respuesta de `GET /users/me/notification-preferences` y `PATCH /users/me/notification-preferences`
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPreferences {
    pub in_app: bool,
    pub email: bool,
}

/// Payload del endpoint `PATCH /users/me/notification-preferences`
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotificationPreferencesRequest {
    pub in_app: Option<bool>,
    pub email: Option<bool>,
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

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Student,
    Professor,
    Admin,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct VerificationToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PasswordResetToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}
