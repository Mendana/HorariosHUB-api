use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::{
        auth::service::password_is_strong,
        users::{
            models::{
                BulkImportResponse, BulkImportRow, BulkImportRowResult, BulkImportRowStatus,
                NotificationPreferences, UpdateNotificationPreferencesRequest, UserPublic,
                UserRole,
            },
            repository::UserRepository,
        },
    },
};

/// Límite de filas por archivo en `POST /users/import`, para evitar subidas abusivas
const MAX_BULK_IMPORT_ROWS: usize = 500;

#[tracing::instrument(skip(repo))]
pub async fn get_all_users(repo: &dyn UserRepository) -> Result<Vec<UserPublic>, AppError> {
    let users = repo.get_all_users().await?;

    let users_public: Vec<UserPublic> = users
        .into_iter()
        .map(|user| UserPublic {
            id: user.id,
            email: user.email,
            role: user.role,
        })
        .collect();

    Ok(users_public)
}

#[tracing::instrument(skip(repo), fields(identifier = %identifier, role = %role))]
pub async fn change_user_role(
    repo: &dyn UserRepository,
    identifier: &str,
    role: &str,
) -> Result<(), AppError> {
    let new_role = match role {
        "student" => UserRole::Student,
        "professor" => UserRole::Professor,
        "admin" => UserRole::Admin,
        _ => return Err(AppError::BadRequest(format!("Invalid role: {role}"))),
    };

    let user_id = match Uuid::parse_str(identifier) {
        Ok(uuid) => uuid,
        Err(_) => {
            tracing::warn!(identifier = %identifier, "UUID de usuario inválido al cambiar rol");
            return Err(AppError::BadRequest(format!(
                "Invalid identifier: {identifier}"
            )));
        }
    };

    repo.change_user_role(user_id, new_role.clone())
        .await
        .map_err(|e| {
            if matches!(e, AppError::NotFound) {
                tracing::warn!(user_id = %user_id, "Usuario no encontrado al cambiar rol");
            }
            e
        })?;

    tracing::info!(user_id = %user_id, new_role = ?new_role, "Rol de usuario cambiado");
    Ok(())
}

#[tracing::instrument(skip(repo), fields(identifier = %identifier))]
pub async fn delete_user(repo: &dyn UserRepository, identifier: &str) -> Result<(), AppError> {
    let user_id = match Uuid::parse_str(identifier) {
        Ok(uuid) => uuid,
        Err(_) => {
            tracing::warn!(identifier = %identifier, "UUID de usuario inválido al eliminar");
            return Err(AppError::BadRequest(format!(
                "Invalid identifier: {identifier}"
            )));
        }
    };

    repo.delete_user(user_id).await.map_err(|e| {
        if matches!(e, AppError::NotFound) {
            tracing::warn!(user_id = %user_id, "Usuario no encontrado al eliminar");
        }
        e
    })?;

    tracing::info!(user_id = %user_id, "Usuario eliminado");
    Ok(())
}

#[tracing::instrument(skip(repo), fields(user_id = %user_id))]
pub async fn get_notification_preferences(
    repo: &dyn UserRepository,
    user_id: Uuid,
) -> Result<NotificationPreferences, AppError> {
    let (in_app, email) = repo
        .get_notification_preferences(user_id)
        .await?
        .ok_or_else(|| {
            tracing::warn!(user_id = %user_id, "Usuario no encontrado al leer preferencias de notificación");
            AppError::NotFound
        })?;

    Ok(NotificationPreferences { in_app, email })
}

#[tracing::instrument(skip(repo, payload), fields(user_id = %user_id))]
pub async fn update_notification_preferences(
    repo: &dyn UserRepository,
    user_id: Uuid,
    payload: UpdateNotificationPreferencesRequest,
) -> Result<NotificationPreferences, AppError> {
    tracing::debug!(user_id = %user_id, "Actualizando preferencias de notificación");

    let (current_in_app, current_email) = repo
        .get_notification_preferences(user_id)
        .await?
        .ok_or_else(|| {
            tracing::warn!(user_id = %user_id, "Usuario no encontrado al actualizar preferencias de notificación");
            AppError::NotFound
        })?;

    let new_in_app = payload.in_app.unwrap_or(current_in_app);
    let new_email = payload.email.unwrap_or(current_email);

    if !new_in_app && !new_email {
        tracing::warn!(user_id = %user_id, "Intento de desactivar ambos canales de notificación a la vez");
        return Err(AppError::BadRequest(
            "Debe mantener activo al menos un canal de notificación".into(),
        ));
    }

    repo.update_notification_preferences(user_id, new_in_app, new_email)
        .await
        .map_err(|e| {
            tracing::error!(?e, user_id = %user_id, "Error al actualizar preferencias de notificación");
            e
        })?;

    tracing::info!(
        user_id = %user_id,
        in_app = new_in_app,
        email = new_email,
        "Preferencias de notificación actualizadas"
    );

    Ok(NotificationPreferences {
        in_app: new_in_app,
        email: new_email,
    })
}

/// Importa en bloque usuarios "default" (cuentas pseudo-generadas como `infprimero`, `matsegundo`, etc.)
/// a partir de un CSV con columnas `email,password`.
///
/// # Flujo:
/// 1. Parsea el CSV fila a fila
/// 2. Por cada fila: valida email/contraseña, comprueba si el email ya existe (si existe, se ignora)
///    y si no, crea el usuario con rol `Student` y lo marca como verificado directamente
/// 3. Nunca aborta por errores de una fila individual: los acumula en `details` y continúa
///
/// # Errores:
/// - [`AppError::BadRequest`] si el archivo supera [`MAX_BULK_IMPORT_ROWS`] filas
/// - [`AppError::Internal`] para errores en hashing o inserción en la base de datos
#[tracing::instrument(skip(repo, csv_content))]
pub async fn bulk_import_users(
    repo: &dyn UserRepository,
    csv_content: &str,
) -> Result<BulkImportResponse, AppError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(csv_content.as_bytes());

    let mut details = Vec::new();
    let mut created = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;

    for (i, result) in reader.deserialize::<BulkImportRow>().enumerate() {
        if details.len() >= MAX_BULK_IMPORT_ROWS {
            tracing::warn!(
                max_rows = MAX_BULK_IMPORT_ROWS,
                "Importación masiva rechazada por exceder el máximo de filas"
            );
            return Err(AppError::BadRequest(format!(
                "El archivo supera el máximo de {MAX_BULK_IMPORT_ROWS} filas"
            )));
        }

        let line = i + 2;
        let row = match result {
            Ok(row) => row,
            Err(e) => {
                failed += 1;
                details.push(BulkImportRowResult {
                    email: format!("fila {line}"),
                    status: BulkImportRowStatus::Error,
                    reason: Some(format!("CSV malformado: {e}")),
                });
                continue;
            }
        };

        let email = row.email.trim().to_lowercase();

        if email.is_empty() || !email.contains('@') {
            failed += 1;
            details.push(BulkImportRowResult {
                email: row.email.clone(),
                status: BulkImportRowStatus::Error,
                reason: Some("Email inválido".into()),
            });
            continue;
        }

        if !password_is_strong(&row.password) {
            failed += 1;
            details.push(BulkImportRowResult {
                email,
                status: BulkImportRowStatus::Error,
                reason: Some(
                    "La contraseña debe tener al menos 8 caracteres, incluir mayúsculas, minúsculas y números"
                        .into(),
                ),
            });
            continue;
        }

        if repo.find_by_email(&email).await?.is_some() {
            tracing::debug!(email = %email, "Usuario ya existe, se ignora en la importación masiva");
            skipped += 1;
            details.push(BulkImportRowResult {
                email,
                status: BulkImportRowStatus::Skipped,
                reason: Some("El usuario ya existe".into()),
            });
            continue;
        }

        let password = row.password.clone();
        let password_hash =
            tokio::task::spawn_blocking(move || bcrypt::hash(password, bcrypt::DEFAULT_COST))
                .await
                .map_err(|e| AppError::Internal(e.into()))?
                .map_err(|e| AppError::Internal(e.into()))?;

        let user = repo
            .create(&email, &password_hash, UserRole::Student)
            .await?;
        repo.mark_user_as_verified(user.id).await?;

        created += 1;
        details.push(BulkImportRowResult {
            email,
            status: BulkImportRowStatus::Created,
            reason: None,
        });
    }

    tracing::info!(
        total = details.len(),
        created,
        skipped,
        failed,
        "Importación masiva de usuarios default completada"
    );

    Ok(BulkImportResponse {
        total: details.len(),
        created,
        skipped,
        failed,
        details,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        errors::AppError,
        modules::users::{
            models::{PasswordResetToken, User, UserRole, VerificationToken},
            repository::UserRepository,
        },
    };
    use async_trait::async_trait;
    use uuid::Uuid;

    struct MockUserRepository {
        users: Vec<User>,
        error: bool,
    }

    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn find_by_id(&self, _id: Uuid) -> Result<Option<User>, AppError> {
            Ok(None)
        }
        async fn find_by_email(&self, _email: &str) -> Result<Option<User>, AppError> {
            Ok(None)
        }
        async fn create(
            &self,
            _email: &str,
            _hash: &str,
            _role: UserRole,
        ) -> Result<User, AppError> {
            unimplemented!()
        }
        async fn create_verification_token(&self, _user_id: Uuid) -> Result<String, AppError> {
            unimplemented!()
        }
        async fn find_verification_token(
            &self,
            _token: &str,
        ) -> Result<Option<VerificationToken>, AppError> {
            unimplemented!()
        }
        async fn mark_user_as_verified(&self, _user_id: Uuid) -> Result<(), AppError> {
            unimplemented!()
        }
        async fn delete_verification_token(&self, _token_id: Uuid) -> Result<(), AppError> {
            unimplemented!()
        }
        async fn create_password_reset_token(&self, _user_id: Uuid) -> Result<String, AppError> {
            unimplemented!()
        }
        async fn find_password_reset_token(
            &self,
            _token: &str,
        ) -> Result<Option<PasswordResetToken>, AppError> {
            unimplemented!()
        }
        async fn update_password(&self, _user_id: Uuid, _hash: &str) -> Result<(), AppError> {
            unimplemented!()
        }
        async fn delete_password_reset_token(&self, _token_id: Uuid) -> Result<(), AppError> {
            unimplemented!()
        }
        async fn get_all_users(&self) -> Result<Vec<User>, AppError> {
            if self.error {
                return Err(AppError::Internal(anyhow::anyhow!("db error")));
            }
            Ok(self.users.clone())
        }
        async fn change_user_role(
            &self,
            _user_id: Uuid,
            _new_role: UserRole,
        ) -> Result<(), AppError> {
            if self.error {
                return Err(AppError::Internal(anyhow::anyhow!("db error")));
            }
            Ok(())
        }
        async fn delete_user(&self, _user_id: Uuid) -> Result<(), AppError> {
            if self.error {
                return Err(AppError::Internal(anyhow::anyhow!("db error")));
            }
            Ok(())
        }
        async fn get_notification_preferences(
            &self,
            _user_id: Uuid,
        ) -> Result<Option<(bool, bool)>, AppError> {
            unimplemented!()
        }
        async fn update_notification_preferences(
            &self,
            _user_id: Uuid,
            _notify_in_app: bool,
            _notify_email: bool,
        ) -> Result<(), AppError> {
            unimplemented!()
        }
    }

    fn make_user(email: &str, role: UserRole) -> User {
        User {
            id: Uuid::new_v4(),
            email: email.to_string(),
            password_hash: "hash".to_string(),
            role,
            verified: true,
        }
    }

    #[tokio::test]
    async fn get_all_users_returns_empty_list() {
        let repo = MockUserRepository {
            users: vec![],
            error: false,
        };
        let result = get_all_users(&repo).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn get_all_users_maps_to_public() {
        let repo = MockUserRepository {
            users: vec![
                make_user("a@uniovi.es", UserRole::Student),
                make_user("b@uniovi.es", UserRole::Professor),
                make_user("c@uniovi.es", UserRole::Admin),
            ],
            error: false,
        };

        let result = get_all_users(&repo).await.unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].email, "a@uniovi.es");
        assert_eq!(result[0].role, UserRole::Student);
        assert_eq!(result[1].email, "b@uniovi.es");
        assert_eq!(result[1].role, UserRole::Professor);
        assert_eq!(result[2].email, "c@uniovi.es");
        assert_eq!(result[2].role, UserRole::Admin);
    }

    #[tokio::test]
    async fn get_all_users_propagates_repo_error() {
        let repo = MockUserRepository {
            users: vec![],
            error: true,
        };
        let result = get_all_users(&repo).await;
        assert!(matches!(result, Err(AppError::Internal(_))));
    }

    // ─── change_user_role ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn change_user_role_returns_ok_for_valid_input() {
        let repo = MockUserRepository {
            users: vec![],
            error: false,
        };
        let id = Uuid::new_v4().to_string();
        let result = change_user_role(&repo, &id, "professor").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn change_user_role_rejects_invalid_role() {
        let repo = MockUserRepository {
            users: vec![],
            error: false,
        };
        let id = Uuid::new_v4().to_string();
        let result = change_user_role(&repo, &id, "superadmin").await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn change_user_role_rejects_invalid_uuid() {
        let repo = MockUserRepository {
            users: vec![],
            error: false,
        };
        let result = change_user_role(&repo, "not-a-uuid", "admin").await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn change_user_role_propagates_repo_error() {
        let repo = MockUserRepository {
            users: vec![],
            error: true,
        };
        let id = Uuid::new_v4().to_string();
        let result = change_user_role(&repo, &id, "student").await;
        assert!(matches!(result, Err(AppError::Internal(_))));
    }

    struct MockNotFoundRepository;

    #[async_trait]
    impl UserRepository for MockNotFoundRepository {
        async fn find_by_id(&self, _id: Uuid) -> Result<Option<User>, AppError> {
            Ok(None)
        }
        async fn find_by_email(&self, _email: &str) -> Result<Option<User>, AppError> {
            Ok(None)
        }
        async fn create(&self, _: &str, _: &str, _: UserRole) -> Result<User, AppError> {
            unimplemented!()
        }
        async fn create_verification_token(&self, _: Uuid) -> Result<String, AppError> {
            unimplemented!()
        }
        async fn find_verification_token(
            &self,
            _: &str,
        ) -> Result<Option<VerificationToken>, AppError> {
            unimplemented!()
        }
        async fn mark_user_as_verified(&self, _: Uuid) -> Result<(), AppError> {
            unimplemented!()
        }
        async fn delete_verification_token(&self, _: Uuid) -> Result<(), AppError> {
            unimplemented!()
        }
        async fn create_password_reset_token(&self, _: Uuid) -> Result<String, AppError> {
            unimplemented!()
        }
        async fn find_password_reset_token(
            &self,
            _: &str,
        ) -> Result<Option<PasswordResetToken>, AppError> {
            unimplemented!()
        }
        async fn update_password(&self, _: Uuid, _: &str) -> Result<(), AppError> {
            unimplemented!()
        }
        async fn delete_password_reset_token(&self, _: Uuid) -> Result<(), AppError> {
            unimplemented!()
        }
        async fn get_all_users(&self) -> Result<Vec<User>, AppError> {
            unimplemented!()
        }
        async fn change_user_role(&self, _: Uuid, _: UserRole) -> Result<(), AppError> {
            Err(AppError::NotFound)
        }
        async fn delete_user(&self, _: Uuid) -> Result<(), AppError> {
            Err(AppError::NotFound)
        }
        async fn get_notification_preferences(
            &self,
            _: Uuid,
        ) -> Result<Option<(bool, bool)>, AppError> {
            Ok(None)
        }
        async fn update_notification_preferences(
            &self,
            _: Uuid,
            _: bool,
            _: bool,
        ) -> Result<(), AppError> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn change_user_role_propagates_not_found() {
        let repo = MockNotFoundRepository;
        let id = Uuid::new_v4().to_string();
        let result = change_user_role(&repo, &id, "admin").await;
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    // ─── bulk_import_users ────────────────────────────────────────────────────

    struct MockBulkImportRepository {
        existing_emails: Vec<String>,
    }

    #[async_trait]
    impl UserRepository for MockBulkImportRepository {
        async fn find_by_id(&self, _id: Uuid) -> Result<Option<User>, AppError> {
            Ok(None)
        }
        async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
            if self.existing_emails.iter().any(|e| e == email) {
                Ok(Some(User {
                    id: Uuid::new_v4(),
                    email: email.to_string(),
                    password_hash: "hash".into(),
                    role: UserRole::Student,
                    verified: true,
                }))
            } else {
                Ok(None)
            }
        }
        async fn create(&self, email: &str, _hash: &str, role: UserRole) -> Result<User, AppError> {
            Ok(User {
                id: Uuid::new_v4(),
                email: email.to_string(),
                password_hash: "hash".into(),
                role,
                verified: false,
            })
        }
        async fn create_verification_token(&self, _: Uuid) -> Result<String, AppError> {
            unimplemented!()
        }
        async fn find_verification_token(
            &self,
            _: &str,
        ) -> Result<Option<VerificationToken>, AppError> {
            unimplemented!()
        }
        async fn mark_user_as_verified(&self, _: Uuid) -> Result<(), AppError> {
            Ok(())
        }
        async fn delete_verification_token(&self, _: Uuid) -> Result<(), AppError> {
            unimplemented!()
        }
        async fn create_password_reset_token(&self, _: Uuid) -> Result<String, AppError> {
            unimplemented!()
        }
        async fn find_password_reset_token(
            &self,
            _: &str,
        ) -> Result<Option<PasswordResetToken>, AppError> {
            unimplemented!()
        }
        async fn update_password(&self, _: Uuid, _: &str) -> Result<(), AppError> {
            unimplemented!()
        }
        async fn delete_password_reset_token(&self, _: Uuid) -> Result<(), AppError> {
            unimplemented!()
        }
        async fn get_all_users(&self) -> Result<Vec<User>, AppError> {
            unimplemented!()
        }
        async fn change_user_role(&self, _: Uuid, _: UserRole) -> Result<(), AppError> {
            unimplemented!()
        }
        async fn delete_user(&self, _: Uuid) -> Result<(), AppError> {
            unimplemented!()
        }
        async fn get_notification_preferences(
            &self,
            _: Uuid,
        ) -> Result<Option<(bool, bool)>, AppError> {
            unimplemented!()
        }
        async fn update_notification_preferences(
            &self,
            _: Uuid,
            _: bool,
            _: bool,
        ) -> Result<(), AppError> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn bulk_import_crea_nuevos_y_omite_existentes() {
        let repo = MockBulkImportRepository {
            existing_emails: vec!["infprimero@uniovi.es".to_string()],
        };
        let csv = "email,password\n\
                   infprimero@uniovi.es,Password123\n\
                   matprimero@uniovi.es,Password123\n";

        let result = bulk_import_users(&repo, csv).await.unwrap();

        assert_eq!(result.total, 2);
        assert_eq!(result.created, 1);
        assert_eq!(result.skipped, 1);
        assert_eq!(result.failed, 0);
        assert_eq!(result.details[0].status, BulkImportRowStatus::Skipped);
        assert_eq!(result.details[1].status, BulkImportRowStatus::Created);
    }

    #[tokio::test]
    async fn bulk_import_marca_error_en_contrasena_debil() {
        let repo = MockBulkImportRepository {
            existing_emails: vec![],
        };
        let csv = "email,password\nmatprimero@uniovi.es,weak\n";

        let result = bulk_import_users(&repo, csv).await.unwrap();

        assert_eq!(result.failed, 1);
        assert_eq!(result.created, 0);
        assert_eq!(result.details[0].status, BulkImportRowStatus::Error);
    }

    #[tokio::test]
    async fn bulk_import_marca_error_en_fila_malformada() {
        let repo = MockBulkImportRepository {
            existing_emails: vec![],
        };
        // Falta la columna "password"
        let csv = "email,password\nmatprimero@uniovi.es\n";

        let result = bulk_import_users(&repo, csv).await.unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.failed, 1);
        assert_eq!(result.details[0].status, BulkImportRowStatus::Error);
    }

    #[tokio::test]
    async fn bulk_import_normaliza_email_a_minusculas() {
        let repo = MockBulkImportRepository {
            existing_emails: vec![],
        };
        let csv = "email,password\nINFPRIMERO@UNIOVI.ES,Password123\n";

        let result = bulk_import_users(&repo, csv).await.unwrap();

        assert_eq!(result.details[0].email, "infprimero@uniovi.es");
        assert_eq!(result.created, 1);
    }

    // ─── delete_user ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_user_returns_ok_for_valid_uuid() {
        let repo = MockUserRepository {
            users: vec![],
            error: false,
        };
        let id = Uuid::new_v4().to_string();
        let result = delete_user(&repo, &id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_user_rejects_invalid_uuid() {
        let repo = MockUserRepository {
            users: vec![],
            error: false,
        };
        let result = delete_user(&repo, "not-a-uuid").await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn delete_user_propagates_repo_error() {
        let repo = MockUserRepository {
            users: vec![],
            error: true,
        };
        let id = Uuid::new_v4().to_string();
        let result = delete_user(&repo, &id).await;
        assert!(matches!(result, Err(AppError::Internal(_))));
    }

    #[tokio::test]
    async fn delete_user_propagates_not_found() {
        let repo = MockNotFoundRepository;
        let id = Uuid::new_v4().to_string();
        let result = delete_user(&repo, &id).await;
        assert!(matches!(result, Err(AppError::NotFound)));
    }
}
