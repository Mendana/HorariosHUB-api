use super::middleware::AuthenticatedUser;
use super::models::{RegisterRequest, RegisterResponse, UserPublic};
use super::service;
use crate::errors::AppError;
use crate::modules::auth::models::{LoginRequest, VerifyEmailQuery, VerifyEmailResponse};
use crate::{AppState, errors::ApiResult};
use axum::extract::Query;
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::IntoResponse;
use axum::{Json, extract::State, http::StatusCode};
use validator::Validate;

/// POST /auth/register
#[tracing::instrument(
    skip(state),                    // No logear el estado completo
    fields(email = %payload.email)  // Logear el email
)]
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> ApiResult<(StatusCode, Json<RegisterResponse>)> {
    payload
        .validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let response = service::register(state.user_repo.as_ref(), payload).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /auth/me
#[tracing::instrument(
    skip(auth),                     // No logear el auth completo
    fields(email = %auth.claims.email)  // Logear el email
)]
pub async fn get_user(auth: AuthenticatedUser) -> ApiResult<(StatusCode, Json<UserPublic>)> {
    // El extractor ya validó el JWT y extrajo los Claims
    // Aquí sacas la información directamente del token
    let response = UserPublic {
        email: auth.claims.email.clone(),
        role: auth.claims.role.clone(),
    };

    Ok((StatusCode::OK, Json(response)))
}

/// POST /auth/login
///
/// Autentica al usuario y establece el JWT en una cookie HttpOnly
#[tracing::instrument(skip(state), fields(email = %payload.email))]
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let (response, token) =
        service::login(state.user_repo.as_ref(), &state.config, payload).await?;

    let cookie_value = format!(
        "access_token={}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}",
        token, state.config.jwt_access_ttl_seconds
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie_value).map_err(|e| AppError::Internal(e.into()))?,
    );

    Ok((headers, Json(response)))
}

/// GET /auth/verify-email?token=abc123
///
/// Verifica el email del usuario utilizando un token de verificación
#[tracing::instrument(skip(state))]
pub async fn verify_email(
    State(state): State<AppState>,
    Query(query): Query<VerifyEmailQuery>,
) -> ApiResult<Json<VerifyEmailResponse>> {
    let response = service::verify_email(state.user_repo.as_ref(), &query.token).await?;

    Ok(Json(response))
}
