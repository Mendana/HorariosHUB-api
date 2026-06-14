use crate::{
    errors::AppError,
    modules::users::{models::UserPublic, repository::UserRepository},
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
}
