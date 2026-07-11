use uuid::Uuid;

use crate::{
    errors::AppError,
    modules::users::{
        models::{UserPublic, UserRole},
        repository::UserRepository,
    },
};

pub async fn get_all_users(repo: &dyn UserRepository) -> Result<Vec<UserPublic>, AppError> {
    let users = repo.get_all_users().await?;

    let users_public: Vec<UserPublic> = users
        .into_iter()
        .map(|user| UserPublic {
            email: user.email,
            role: user.role,
        })
        .collect();

    Ok(users_public)
}

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
    }

    #[tokio::test]
    async fn change_user_role_propagates_not_found() {
        let repo = MockNotFoundRepository;
        let id = Uuid::new_v4().to_string();
        let result = change_user_role(&repo, &id, "admin").await;
        assert!(matches!(result, Err(AppError::NotFound)));
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
