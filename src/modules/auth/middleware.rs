use crate::AppState;
use crate::errors::AppError;
use crate::jwt;
use crate::modules::users::models::{User, UserRole};
use axum::extract::{FromRef, FromRequestParts};
use axum::http::header::{AUTHORIZATION, COOKIE};
use axum::http::request::Parts;
use uuid::Uuid;

/// Extractor para cualquier usuario autenticado y verificado.
/// Valida el JWT, consulta la DB para obtener datos frescos y comprueba que el usuario esté verificado.
pub struct AuthenticatedUser {
    pub user: User,
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);

        let token = extract_bearer(parts).or_else(|| extract_cookie(parts, "access_token"));
        let token = token.ok_or({
            tracing::warn!("No se encontró token de autenticación en la cabecera ni en la cookie");
            AppError::Unauthorized
        })?;

        let claims = jwt::verify_token(&token, &state.config.jwt_secret).map_err(|e| {
            tracing::warn!(error = ?e, "JWT inválido o expirado");
            AppError::Unauthorized
        })?;
        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;

        let user = state.user_repo.find_by_id(user_id).await?.ok_or({
            tracing::warn!(
                user_id = %user_id,
                "JWT válido pero usuario no existe en BBDD"
            );
            AppError::Unauthorized
        })?;

        if !user.verified {
            tracing::warn!(
                user_id = %user_id,
                "Usuario no verificado"
            );
            return Err(AppError::Forbidden);
        }

        tracing::debug!(
            user_id = %user_id,
            email = %user.email,
            role = ?user.role,
            "Usuario autenticado y verificado"
        );
        Ok(AuthenticatedUser { user })
    }
}

/// Extractor que además requiere rol Admin.
pub struct AdminUser(pub User);

impl<S> FromRequestParts<S> for AdminUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = AuthenticatedUser::from_request_parts(parts, state).await?;

        if auth.user.role != UserRole::Admin {
            tracing::warn!(
                user_id = %auth.user.id,
                email = %auth.user.email,
                role = ?auth.user.role,
                "Usuario autenticado pero no tiene rol Admin"
            );
            return Err(AppError::Forbidden);
        }

        tracing::debug!(
            user_id = %auth.user.id,
            email = %auth.user.email,
            role = ?auth.user.role,
            "Usuario autenticado con rol Admin"
        );
        Ok(AdminUser(auth.user))
    }
}

/// Extractor que además requiere rol Professor o superior.
pub struct ProfessorOrAbove(pub User);

impl<S> FromRequestParts<S> for ProfessorOrAbove
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = AuthenticatedUser::from_request_parts(parts, state).await?;

        match auth.user.role {
            UserRole::Professor | UserRole::Admin => {
                tracing::debug!(
                    user_id = %auth.user.id,
                    email = %auth.user.email,
                    role = ?auth.user.role,
                    "Usuario autenticado con rol Professor o superior"
                );
                Ok(ProfessorOrAbove(auth.user))
            }
            _ => {
                tracing::warn!(
                    user_id = %auth.user.id,
                    email = %auth.user.email,
                    role = ?auth.user.role,
                    "Usuario autenticado pero no tiene rol Professor o superior"
                );
                Err(AppError::Forbidden)
            }
        }
    }
}

fn extract_bearer(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

fn extract_cookie(parts: &Parts, name: &str) -> Option<String> {
    let header = parts.headers.get(COOKIE)?.to_str().ok()?;
    header.split(';').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        if k.trim() == name {
            Some(v.trim().to_string())
        } else {
            None
        }
    })
}
