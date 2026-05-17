use chrono::Utc;

use super::models::{RegisterRequest, RegisterResponse, UserRole};
use crate::config::Config;
use crate::errors::AppError;
use crate::jwt;
use crate::modules::auth::models::{LoginRequest, LoginResponse, UserPublic, VerifyEmailResponse};
use crate::modules::auth::repository::UserRepository;

/// Registra un nuevo usuario en el sistema
///
/// # Flujo:
/// 1. Verifica que el email no esté registrado
/// 2. Hashea la contraseña
/// 3. Crea el usuario en la base de datos con el rol de `Student` por defecto
///
/// # Errores:
/// - [`AppError::Conflict`] si el email ya está registrado
/// - [`AppError::Internal`] para errores en hashing o inserción en la base de datos
pub async fn register(
    repo: &dyn UserRepository,
    payload: RegisterRequest,
) -> Result<RegisterResponse, AppError> {
    let email = payload.email.trim().to_lowercase();

    if repo.find_by_email(&email).await?.is_some() {
        return Err(AppError::Conflict("El email ya está registrado".into()));
    }

    let password = payload.password.clone();
    let password_hash =
        tokio::task::spawn_blocking(move || bcrypt::hash(password, bcrypt::DEFAULT_COST))
            .await
            .map_err(|e| AppError::Internal(e.into()))?
            .map_err(|e| AppError::Internal(e.into()))?;

    let user = repo
        .create(&email, &password_hash, UserRole::Student)
        .await?;

    repo.create_verification_token(user.id).await?;

    Ok(RegisterResponse {
        id: user.id,
        email: user.email,
        role: user.role,
    })
}

/// Realiza el login de un usuario y genera un JWT de acceso si las credenciales son correctas
///
/// # Flujo:
/// 1. Busca el usuario por email
/// 2. Verifica que la contraseña sea correcta
/// 3. Genera un JWT con la información del usuario y el rol
///
/// # Errores:
/// - [`AppError::Unauthorized`] si el email no existe o la contraseña es incorrecta
/// - [`AppError::Internal`] para errores en verificación de contraseña o generación de token
pub async fn login(
    repo: &dyn UserRepository,
    config: &Config,
    payload: LoginRequest,
) -> Result<(LoginResponse, String), AppError> {
    let email = payload.email.trim().to_lowercase();

    let user = repo
        .find_by_email(&email)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let password = payload.password.clone();
    let hash = user.password_hash.clone();
    let valid = tokio::task::spawn_blocking(move || bcrypt::verify(password, &hash))
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .map_err(|e| AppError::Internal(e.into()))?;

    if !valid {
        return Err(AppError::Unauthorized);
    }

    let token = jwt::generate_token(
        &user.id.to_string(),
        &user.email,
        &user.role,
        &config.jwt_secret,
        config.jwt_access_ttl_seconds,
    )?;

    let response = LoginResponse {
        user: UserPublic {
            email: user.email,
            role: user.role,
        },
    };

    Ok((response, token))
}

/// Verifica el email de un usuario utilizando un token de verificación
///
/// # Flujo:
/// 1. Busca el token de verificación en la base de datos
/// 2. Verifica que el token no haya expirado
/// 3. Marca el usuario como verificado
/// 4. Elimina el token de verificación
///
/// # Errores:
/// - [`AppError::BadRequest`] si el token es inválido, ya utilizado o ha expirado
/// - [`AppError::Internal`] para errores en la base de datos o en la lógica de verificación
pub async fn verify_email(
    repo: &dyn UserRepository,
    token: &str,
) -> Result<VerifyEmailResponse, AppError> {
    let verification = repo
        .find_verification_token(token)
        .await?
        .ok_or_else(|| AppError::BadRequest("Token inválido o ya utilizado".into()))?;

    if verification.expires_at < Utc::now() {
        repo.delete_verification_token(verification.id).await?;
        return Err(AppError::BadRequest("El token ha expirado".into()));
    }

    repo.mark_user_as_verified(verification.user_id).await?;

    repo.delete_verification_token(verification.id).await?;

    Ok(VerifyEmailResponse {
        message: "Email verificado correctamente".into(),
    })
}

// --- Testing ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::AppError;
    use crate::modules::auth::models::{RegisterRequest, User, UserRole, VerificationToken};
    use crate::modules::auth::repository::UserRepository;
    use async_trait::async_trait;
    use uuid::Uuid;

    // Mock del repository para tests
    struct MockUserRepository {
        existing_email: Option<String>,
    }

    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
            if self.existing_email.as_deref() == Some(email) {
                Ok(Some(User {
                    id: Uuid::new_v4(),
                    email: email.to_string(),
                    password_hash: "hash".to_string(),
                    role: UserRole::Student,
                    verified: false,
                }))
            } else {
                Ok(None)
            }
        }

        async fn create(
            &self,
            email: &str,
            _password_hash: &str,
            role: UserRole,
        ) -> Result<User, AppError> {
            Ok(User {
                id: Uuid::new_v4(),
                email: email.to_string(),
                password_hash: "hash".to_string(),
                role,
                verified: false,
            })
        }

        async fn create_verification_token(&self, _user_id: Uuid) -> Result<String, AppError> {
            Ok("token".to_string())
        }

        async fn delete_verification_token(&self, _token_id: Uuid) -> Result<(), AppError> {
            Ok(())
        }

        async fn find_verification_token(
            &self,
            _token: &str,
        ) -> Result<Option<VerificationToken>, AppError> {
            Ok(Some(VerificationToken {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                token: "token".to_string(),
                expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
            }))
        }

        async fn mark_user_as_verified(&self, _user_id: Uuid) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn register_ok() {
        let repo = MockUserRepository {
            existing_email: None,
        };
        let payload = RegisterRequest {
            email: "diego@uniovi.es".to_string(),
            password: "password123".to_string(),
        };

        let result = register(&repo, payload).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.email, "diego@uniovi.es");
        assert_eq!(response.role, UserRole::Student);
    }

    #[tokio::test]
    async fn register_normaliza_email_a_minusculas() {
        let repo = MockUserRepository {
            existing_email: None,
        };
        let payload = RegisterRequest {
            email: "DIEGO@UNIOVI.ES".to_string(),
            password: "password123".to_string(),
        };

        let result = register(&repo, payload).await.unwrap();
        assert_eq!(result.email, "diego@uniovi.es");
    }

    #[tokio::test]
    async fn register_falla_si_email_duplicado() {
        let repo = MockUserRepository {
            existing_email: Some("diego@uniovi.es".to_string()),
        };
        let payload = RegisterRequest {
            email: "diego@uniovi.es".to_string(),
            password: "password123".to_string(),
        };

        let result = register(&repo, payload).await;

        assert!(matches!(result, Err(AppError::Conflict(_))));
    }
}
