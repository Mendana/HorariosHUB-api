use chrono::Utc;

use super::models::{RegisterRequest, RegisterResponse, UserRole};
use crate::config::Config;
use crate::errors::AppError;
use crate::jwt;
use crate::modules::auth::models::{
    LoginRequest, LoginResponse, RecoverPasswordRequest, RecoverPasswordResponse,
    ResetPasswordRequest, ResetPasswordResponse, UserPublic, VerifyEmailResponse,
};
use crate::modules::auth::repository::UserRepository;
use crate::services::email::service::EmailService;

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
    email_svc: &dyn EmailService,
    payload: RegisterRequest,
) -> Result<RegisterResponse, AppError> {
    if !password_is_strong(&payload.password) {
        return Err(AppError::Validation(
            "La contraseña debe tener al menos 8 caracteres, incluir mayúsculas, minúsculas y números".into(),
        ));
    }

    let email = payload.email.trim().to_lowercase();

    if !email.ends_with("@uniovi.es") {
        return Err(AppError::Validation(
            "El email debe pertenecer al dominio @uniovi.es".into(),
        ));
    }

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

    let token = repo.create_verification_token(user.id).await?;

    if let Err(e) = email_svc.send_verification_email(&email, &token).await {
        tracing::error!(
            email = %email,
            error = ?e,
            "No se pudo enviar el email de verificacion"
        );
    }

    Ok(RegisterResponse {
        email: user.email,
        role: user.role,
    })
}

fn password_is_strong(password: &str) -> bool {
    let has_min_length = password.len() >= 8;
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_numeric());

    has_min_length && has_uppercase && has_lowercase && has_digit
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

pub async fn reset_password(
    repo: &dyn UserRepository,
    payload: ResetPasswordRequest,
) -> Result<ResetPasswordResponse, AppError> {
    let reset_token = repo
        .find_password_reset_token(&payload.token)
        .await?
        .ok_or_else(|| AppError::BadRequest("Token inválido o ya utilizado".into()))?;

    if reset_token.expires_at < Utc::now() {
        repo.delete_password_reset_token(reset_token.id).await?;
        return Err(AppError::BadRequest("El token ha expirado".into()));
    }

    let password = payload.new_password.clone();

    if !password_is_strong(&password) {
        return Err(AppError::Validation(
            "La contraseña debe tener al menos 8 caracteres, incluir mayúsculas, minúsculas y números".into(),
        ));
    }

    let password_hash =
        tokio::task::spawn_blocking(move || bcrypt::hash(password, bcrypt::DEFAULT_COST))
            .await
            .map_err(|e| AppError::Internal(e.into()))?
            .map_err(|e| AppError::Internal(e.into()))?;

    repo.update_password(reset_token.user_id, &password_hash)
        .await?;

    repo.delete_password_reset_token(reset_token.id).await?;

    Ok(ResetPasswordResponse {
        message: "Contraseña actualizada".into(),
    })
}

/// Inicia el proceso de recuperación de contraseña para un email dado
///
/// # Flujo:
/// 1. Busca el usuario por email
/// 2. Si el usuario existe, genera un token de recuperación de contraseña
/// 3. Envía un email al usuario con el enlace de recuperación (contiene el token)
/// 4. Si el email no existe, no hace nada (para evitar revelar qué emails están registrados en el sistema)
/// 5. Devuelve un mensaje genérico indicando que se ha enviado un enlace de recuperación (independientemente de si el email existe o no)
///
/// # Errores:
/// - [`AppError::Internal`] para errores en la base de datos o en el envío de emails
pub async fn recover_password(
    repo: &dyn UserRepository,
    email_svc: &dyn EmailService,
    payload: RecoverPasswordRequest,
) -> Result<RecoverPasswordResponse, AppError> {
    let email = payload.email.trim().to_lowercase();

    let user = match repo.find_by_email(&email).await? {
        Some(user) => user,
        None => {
            tracing::debug!(
            email = %email,
            "Recover solicitado para email no registrado"
            );
            return Ok(RecoverPasswordResponse {
                message: "Si el email existe, se ha enviado un enlace de recuperación".into(),
            });
        }
    };

    let token = repo.create_password_reset_token(user.id).await?;

    if let Err(e) = email_svc.send_password_reset_email(&email, &token).await {
        tracing::error!(
        email = %email,
        error = ?e,
        "No se pudo enviar el email de recuperación"
        );
    }

    Ok(RecoverPasswordResponse {
        message: "Si el email existe, se ha enviado un enlace de recuperación".into(),
    })
}

// --- Testing ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::AppError;
    use crate::modules::auth::models::{
        PasswordResetToken, RegisterRequest, User, UserRole, VerificationToken,
    };
    use crate::modules::auth::repository::UserRepository;
    use crate::services::email::service::MockEmailService;
    use async_trait::async_trait;
    use uuid::Uuid;

    // Mock del repository para tests
    struct MockUserRepository {
        existing_email: Option<String>,
    }

    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn find_by_id(&self, _id: Uuid) -> Result<Option<User>, AppError> {
            Ok(None)
        }

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

        async fn find_password_reset_token(
            &self,
            _token: &str,
        ) -> Result<Option<PasswordResetToken>, AppError> {
            Ok(Some(PasswordResetToken {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                token: "reset_token".to_string(),
                expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
            }))
        }

        async fn delete_password_reset_token(&self, _token_id: Uuid) -> Result<(), AppError> {
            Ok(())
        }

        async fn update_password(
            &self,
            _user_id: Uuid,
            _password_hash: &str,
        ) -> Result<(), AppError> {
            Ok(())
        }

        async fn create_password_reset_token(&self, _user_id: Uuid) -> Result<String, AppError> {
            Ok("reset_token".to_string())
        }
    }

    fn mock_email() -> MockEmailService {
        MockEmailService
    }

    #[tokio::test]
    async fn register_ok() {
        let repo = MockUserRepository {
            existing_email: None,
        };
        let payload = RegisterRequest {
            email: "diego@uniovi.es".to_string(),
            password: "Password123".to_string(),
        };

        let result = register(&repo, &mock_email(), payload).await;

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
            password: "Password123".to_string(),
        };

        let result = register(&repo, &mock_email(), payload).await.unwrap();
        assert_eq!(result.email, "diego@uniovi.es");
    }

    #[tokio::test]
    async fn register_falla_si_email_duplicado() {
        let repo = MockUserRepository {
            existing_email: Some("diego@uniovi.es".to_string()),
        };
        let payload = RegisterRequest {
            email: "diego@uniovi.es".to_string(),
            password: "Password123".to_string(),
        };

        let result = register(&repo, &mock_email(), payload).await;

        assert!(matches!(result, Err(AppError::Conflict(_))));
    }

    #[tokio::test]
    async fn register_falla_si_contraseña_debil() {
        let repo = MockUserRepository {
            existing_email: None,
        };
        let no_uppercase_password = RegisterRequest {
            email: "diego@uniovi.es".to_string(),
            password: "password123".to_string(),
        };
        let no_lowercase_password = RegisterRequest {
            email: "diego@uniovi.es".to_string(),
            password: "PASSWORD123".to_string(),
        };
        let no_numbers_password = RegisterRequest {
            email: "diego@uniovi.es".to_string(),
            password: "Password".to_string(),
        };
        let short_password = RegisterRequest {
            email: "diego@uniovi.es".to_string(),
            password: "Pass1".to_string(),
        };

        let result = register(&repo, &mock_email(), no_uppercase_password).await;
        assert!(matches!(result, Err(AppError::Validation(_))));

        let result = register(&repo, &mock_email(), no_lowercase_password).await;
        assert!(matches!(result, Err(AppError::Validation(_))));

        let result = register(&repo, &mock_email(), no_numbers_password).await;
        assert!(matches!(result, Err(AppError::Validation(_))));

        let result = register(&repo, &mock_email(), short_password).await;
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    struct MockUserRepositoryLogin {
        existing_email: Option<String>,
        password_hash: String,
    }

    #[async_trait]
    impl UserRepository for MockUserRepositoryLogin {
        async fn find_by_id(&self, _id: Uuid) -> Result<Option<User>, AppError> {
            Ok(None)
        }

        async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
            if self.existing_email.as_deref() == Some(email) {
                Ok(Some(User {
                    id: Uuid::new_v4(),
                    email: email.to_string(),
                    password_hash: self.password_hash.clone(),
                    role: UserRole::Student,
                    verified: true,
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
                password_hash: self.password_hash.clone(),
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

        async fn find_password_reset_token(
            &self,
            _token: &str,
        ) -> Result<Option<PasswordResetToken>, AppError> {
            Ok(None)
        }

        async fn delete_password_reset_token(&self, _token_id: Uuid) -> Result<(), AppError> {
            Ok(())
        }

        async fn update_password(
            &self,
            _user_id: Uuid,
            _password_hash: &str,
        ) -> Result<(), AppError> {
            Ok(())
        }

        async fn create_password_reset_token(&self, _user_id: Uuid) -> Result<String, AppError> {
            Ok("reset_token".to_string())
        }
    }

    #[tokio::test]
    async fn login_ok() {
        // Generar hash válido de "password123" en el test
        let password_hash =
            tokio::task::spawn_blocking(|| bcrypt::hash("password123", bcrypt::DEFAULT_COST))
                .await
                .unwrap()
                .unwrap();

        let repo = MockUserRepositoryLogin {
            existing_email: Some("diego@uniovi.es".to_string()),
            password_hash,
        };
        let config = Config {
            database_url: "postgres://localhost/test".to_string(),
            jwt_secret: "secret".to_string(),
            jwt_access_ttl_seconds: 900,
            server_port: 3001,
            rust_env: crate::config::Environment::Development,
            smtp_host: "localhost".to_string(),
            smtp_port: 1025,
            smtp_user: "test".to_string(),
            smtp_password: "test".to_string(),
            smtp_from: "no-reply@horarioshub.com".to_string(),
            base_url: "http://localhost:3000".to_string(),
        };
        let payload = LoginRequest {
            email: "diego@uniovi.es".to_string(),
            password: "password123".to_string(),
        };

        let result = login(&repo, &config, payload).await;

        assert!(result.is_ok());
        let (response, _token) = result.unwrap();
        assert_eq!(response.user.email, "diego@uniovi.es");
        assert_eq!(response.user.role, UserRole::Student);
    }

    #[tokio::test]
    async fn login_falla_con_email_invalido() {
        let password_hash =
            tokio::task::spawn_blocking(|| bcrypt::hash("password123", bcrypt::DEFAULT_COST))
                .await
                .unwrap()
                .unwrap();

        let repo = MockUserRepositoryLogin {
            existing_email: Some("diego@uniovi.es".to_string()),
            password_hash,
        };
        let config = Config {
            database_url: "postgres://localhost/test".to_string(),
            jwt_secret: "secret".to_string(),
            jwt_access_ttl_seconds: 900,
            server_port: 3001,
            rust_env: crate::config::Environment::Development,
            smtp_host: "localhost".to_string(),
            smtp_port: 1025,
            smtp_user: "test".to_string(),
            smtp_password: "test".to_string(),
            smtp_from: "no-reply@horarioshub.com".to_string(),
            base_url: "http://localhost:3000".to_string(),
        };
        let payload = LoginRequest {
            email: "otro@uniovi.es".to_string(),
            password: "password123".to_string(),
        };

        let result = login(&repo, &config, payload).await;

        assert!(matches!(result, Err(AppError::Unauthorized)));
    }

    #[tokio::test]
    async fn login_falla_con_contraseña_incorrecta() {
        let password_hash =
            tokio::task::spawn_blocking(|| bcrypt::hash("password123", bcrypt::DEFAULT_COST))
                .await
                .unwrap()
                .unwrap();

        let repo = MockUserRepositoryLogin {
            existing_email: Some("diego@uniovi.es".to_string()),
            password_hash,
        };
        let config = Config {
            database_url: "postgres://localhost/test".to_string(),
            jwt_secret: "secret".to_string(),
            jwt_access_ttl_seconds: 900,
            server_port: 3001,
            rust_env: crate::config::Environment::Development,
            smtp_host: "localhost".to_string(),
            smtp_port: 1025,
            smtp_user: "test".to_string(),
            smtp_password: "test".to_string(),
            smtp_from: "no-reply@horarioshub.com".to_string(),
            base_url: "http://localhost:3000".to_string(),
        };
        let payload = LoginRequest {
            email: "diego@uniovi.es".to_string(),
            password: "contraseña_incorrecta".to_string(),
        };

        let result = login(&repo, &config, payload).await;

        assert!(matches!(result, Err(AppError::Unauthorized)));
    }

    #[tokio::test]
    async fn verify_email_ok() {
        let repo = MockUserRepository {
            existing_email: None,
        };

        let result = verify_email(&repo, "token").await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.message, "Email verificado correctamente");
    }

    #[tokio::test]
    async fn verify_email_falla_token_expirado() {
        let _repo = MockUserRepository {
            existing_email: None,
        };

        // Modificamos el mock para simular un token expirado
        struct MockUserRepositoryExpired;

        #[async_trait]
        impl UserRepository for MockUserRepositoryExpired {
            async fn find_by_id(&self, _id: Uuid) -> Result<Option<User>, AppError> {
                Ok(None)
            }

            async fn find_by_email(&self, _email: &str) -> Result<Option<User>, AppError> {
                Ok(None)
            }

            async fn create(
                &self,
                _email: &str,
                _password_hash: &str,
                _role: UserRole,
            ) -> Result<User, AppError> {
                Ok(User {
                    id: Uuid::new_v4(),
                    email: "test@test.com".to_string(),
                    password_hash: "hash".to_string(),
                    role: UserRole::Student,
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
                    expires_at: chrono::Utc::now() - chrono::Duration::hours(1),
                }))
            }

            async fn mark_user_as_verified(&self, _user_id: Uuid) -> Result<(), AppError> {
                Ok(())
            }

            async fn find_password_reset_token(
                &self,
                _token: &str,
            ) -> Result<Option<PasswordResetToken>, AppError> {
                Ok(None)
            }

            async fn delete_password_reset_token(&self, _token_id: Uuid) -> Result<(), AppError> {
                Ok(())
            }

            async fn update_password(
                &self,
                _user_id: Uuid,
                _password_hash: &str,
            ) -> Result<(), AppError> {
                Ok(())
            }

            async fn create_password_reset_token(
                &self,
                _user_id: Uuid,
            ) -> Result<String, AppError> {
                Ok("reset_token".to_string())
            }
        }

        let repo = MockUserRepositoryExpired;
        let result = verify_email(&repo, "token").await;

        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn verify_email_falla_token_invalido() {
        struct MockUserRepositoryInvalidToken;

        #[async_trait]
        impl UserRepository for MockUserRepositoryInvalidToken {
            async fn find_by_id(&self, _id: Uuid) -> Result<Option<User>, AppError> {
                Ok(None)
            }

            async fn find_by_email(&self, _email: &str) -> Result<Option<User>, AppError> {
                Ok(None)
            }

            async fn create(
                &self,
                _email: &str,
                _password_hash: &str,
                _role: UserRole,
            ) -> Result<User, AppError> {
                Ok(User {
                    id: Uuid::new_v4(),
                    email: "test@test.com".to_string(),
                    password_hash: "hash".to_string(),
                    role: UserRole::Student,
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
                Ok(None)
            }

            async fn mark_user_as_verified(&self, _user_id: Uuid) -> Result<(), AppError> {
                Ok(())
            }

            async fn find_password_reset_token(
                &self,
                _token: &str,
            ) -> Result<Option<PasswordResetToken>, AppError> {
                Ok(None)
            }

            async fn delete_password_reset_token(&self, _token_id: Uuid) -> Result<(), AppError> {
                Ok(())
            }

            async fn update_password(
                &self,
                _user_id: Uuid,
                _password_hash: &str,
            ) -> Result<(), AppError> {
                Ok(())
            }

            async fn create_password_reset_token(
                &self,
                _user_id: Uuid,
            ) -> Result<String, AppError> {
                Ok("reset_token".to_string())
            }
        }

        let repo = MockUserRepositoryInvalidToken;
        let result = verify_email(&repo, "invalid_token").await;

        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn reset_password_ok() {
        let repo = MockUserRepository {
            existing_email: None,
        };

        let payload = ResetPasswordRequest {
            token: "reset_token".to_string(),
            new_password: "Newpassword123".to_string(),
        };

        let result = reset_password(&repo, payload).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.message, "Contraseña actualizada");
    }

    #[tokio::test]
    async fn reset_password_falla_token_expirado() {
        struct MockUserRepositoryResetExpired;

        #[async_trait]
        impl UserRepository for MockUserRepositoryResetExpired {
            async fn find_by_id(&self, _id: Uuid) -> Result<Option<User>, AppError> {
                Ok(None)
            }

            async fn find_by_email(&self, _email: &str) -> Result<Option<User>, AppError> {
                Ok(None)
            }

            async fn create(
                &self,
                _email: &str,
                _password_hash: &str,
                _role: UserRole,
            ) -> Result<User, AppError> {
                Ok(User {
                    id: Uuid::new_v4(),
                    email: "test@test.com".to_string(),
                    password_hash: "hash".to_string(),
                    role: UserRole::Student,
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
                Ok(None)
            }

            async fn mark_user_as_verified(&self, _user_id: Uuid) -> Result<(), AppError> {
                Ok(())
            }

            async fn find_password_reset_token(
                &self,
                _token: &str,
            ) -> Result<Option<PasswordResetToken>, AppError> {
                Ok(Some(PasswordResetToken {
                    id: Uuid::new_v4(),
                    user_id: Uuid::new_v4(),
                    token: "reset_token".to_string(),
                    expires_at: chrono::Utc::now() - chrono::Duration::hours(1),
                }))
            }

            async fn delete_password_reset_token(&self, _token_id: Uuid) -> Result<(), AppError> {
                Ok(())
            }

            async fn update_password(
                &self,
                _user_id: Uuid,
                _password_hash: &str,
            ) -> Result<(), AppError> {
                Ok(())
            }

            async fn create_password_reset_token(
                &self,
                _user_id: Uuid,
            ) -> Result<String, AppError> {
                Ok("reset_token".to_string())
            }
        }

        let repo = MockUserRepositoryResetExpired;
        let payload = ResetPasswordRequest {
            token: "reset_token".to_string(),
            new_password: "newpassword123".to_string(),
        };

        let result = reset_password(&repo, payload).await;

        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn reset_password_falla_contraseña_debil() {
        let repo = MockUserRepository {
            existing_email: None,
        };

        let short_password = ResetPasswordRequest {
            token: "reset_token".to_string(),
            new_password: "weak".to_string(),
        };
        let no_uppercase_password = ResetPasswordRequest {
            token: "reset_token".to_string(),
            new_password: "weakpassword1".to_string(),
        };
        let no_lowercase_password = ResetPasswordRequest {
            token: "reset_token".to_string(),
            new_password: "WEAKPASSWORD1".to_string(),
        };
        let no_numbers_password = ResetPasswordRequest {
            token: "reset_token".to_string(),
            new_password: "WeakPassword".to_string(),
        };

        let result = reset_password(&repo, short_password).await;
        assert!(matches!(result, Err(AppError::Validation(_))));

        let result = reset_password(&repo, no_uppercase_password).await;
        assert!(matches!(result, Err(AppError::Validation(_))));

        let result = reset_password(&repo, no_lowercase_password).await;
        assert!(matches!(result, Err(AppError::Validation(_))));

        let result = reset_password(&repo, no_numbers_password).await;
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[tokio::test]
    async fn register_recorta_espacios_en_email() {
        let repo = MockUserRepository {
            existing_email: None,
        };
        let payload = RegisterRequest {
            email: "  diego@uniovi.es  ".to_string(),
            password: "Password123".to_string(),
        };
        let result = register(&repo, &mock_email(), payload).await.unwrap();
        assert_eq!(result.email, "diego@uniovi.es");
    }

    #[tokio::test]
    async fn login_normaliza_email_a_minusculas() {
        let password_hash =
            tokio::task::spawn_blocking(|| bcrypt::hash("password123", bcrypt::DEFAULT_COST))
                .await
                .unwrap()
                .unwrap();

        let repo = MockUserRepositoryLogin {
            existing_email: Some("diego@uniovi.es".to_string()),
            password_hash,
        };
        let config = Config {
            database_url: "postgres://localhost/test".to_string(),
            jwt_secret: "secret".to_string(),
            jwt_access_ttl_seconds: 900,
            server_port: 3001,
            rust_env: crate::config::Environment::Development,
            smtp_host: "localhost".to_string(),
            smtp_port: 1025,
            smtp_user: "test".to_string(),
            smtp_password: "test".to_string(),
            smtp_from: "no-reply@horarioshub.com".to_string(),
            base_url: "http://localhost:3000".to_string(),
        };
        let payload = LoginRequest {
            email: "DIEGO@UNIOVI.ES".to_string(),
            password: "password123".to_string(),
        };

        let result = login(&repo, &config, payload).await;

        assert!(result.is_ok());
        let (response, _token) = result.unwrap();
        assert_eq!(response.user.email, "diego@uniovi.es");
    }

    #[tokio::test]
    async fn reset_password_falla_token_invalido() {
        struct MockUserRepositoryResetInvalid;

        #[async_trait]
        impl UserRepository for MockUserRepositoryResetInvalid {
            async fn find_by_id(&self, _id: Uuid) -> Result<Option<User>, AppError> {
                Ok(None)
            }

            async fn find_by_email(&self, _email: &str) -> Result<Option<User>, AppError> {
                Ok(None)
            }

            async fn create(
                &self,
                _email: &str,
                _password_hash: &str,
                _role: UserRole,
            ) -> Result<User, AppError> {
                Ok(User {
                    id: Uuid::new_v4(),
                    email: "test@test.com".to_string(),
                    password_hash: "hash".to_string(),
                    role: UserRole::Student,
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
                Ok(None)
            }

            async fn mark_user_as_verified(&self, _user_id: Uuid) -> Result<(), AppError> {
                Ok(())
            }

            async fn find_password_reset_token(
                &self,
                _token: &str,
            ) -> Result<Option<PasswordResetToken>, AppError> {
                Ok(None)
            }

            async fn delete_password_reset_token(&self, _token_id: Uuid) -> Result<(), AppError> {
                Ok(())
            }

            async fn update_password(
                &self,
                _user_id: Uuid,
                _password_hash: &str,
            ) -> Result<(), AppError> {
                Ok(())
            }

            async fn create_password_reset_token(
                &self,
                _user_id: Uuid,
            ) -> Result<String, AppError> {
                Ok("reset_token".to_string())
            }
        }

        let repo = MockUserRepositoryResetInvalid;
        let payload = ResetPasswordRequest {
            token: "invalid_token".to_string(),
            new_password: "newpassword123".to_string(),
        };

        let result = reset_password(&repo, payload).await;

        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }
}
