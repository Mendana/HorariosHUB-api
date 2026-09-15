use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::StatusCode,
};

use crate::{
    AppState,
    errors::{ApiResult, AppError},
    modules::{
        auth::middleware::{AdminUser, AuthenticatedUser, ProfessorOrAbove},
        users::{
            models::{
                BulkImportResponse, NotificationPreferences,
                UpdateNotificationPreferencesRequest, UsersListResponse,
            },
            service,
        },
    },
};

#[tracing::instrument(
    name = "Get all users",
    skip(state, professor),
    fields(user_email = %professor.email, user_role = ?professor.role)
)]
pub async fn get_all_users(
    State(state): State<AppState>,
    ProfessorOrAbove(professor): ProfessorOrAbove,
) -> ApiResult<(StatusCode, Json<UsersListResponse>)> {
    let users = service::get_all_users(state.user_repo.as_ref()).await?;

    Ok((StatusCode::OK, Json(UsersListResponse { users })))
}

#[tracing::instrument(
    name = "Change user role",
    skip(state, admin),
    fields(user_email = %admin.email, user_role = ?admin.role, target_id = %identifier, new_role = %role)
)]
pub async fn change_user_role(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path((identifier, role)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    service::change_user_role(state.user_repo.as_ref(), &identifier, &role).await?;

    Ok(StatusCode::OK)
}

#[tracing::instrument(
    name = "Delete user",
    skip(state, admin),
    fields(user_email = %admin.email, user_role = ?admin.role, target_id = %identifier)
)]
pub async fn delete_user(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(identifier): Path<String>,
) -> ApiResult<StatusCode> {
    service::delete_user(state.user_repo.as_ref(), &identifier).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /users/import
///
/// Sube un CSV (`email,password`) con cuentas "default" (p.ej. `infprimero`, `matsegundo`)
/// y las crea ya verificadas, ignorando las que ya existan.
#[tracing::instrument(
    name = "Import default users",
    skip(state, admin, multipart),
    fields(user_email = %admin.email, user_role = ?admin.role)
)]
pub async fn import_default_users(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    mut multipart: Multipart,
) -> ApiResult<Json<BulkImportResponse>> {
    let mut csv_content: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart inválido: {e}")))?
    {
        if field.name() != Some("file") {
            continue;
        }

        let bytes = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("No se pudo leer el archivo: {e}")))?;

        csv_content = Some(
            String::from_utf8(bytes.to_vec())
                .map_err(|_| AppError::BadRequest("El archivo no es UTF-8 válido".into()))?,
        );
    }

    let csv_content = csv_content
        .ok_or_else(|| AppError::BadRequest("Falta el campo 'file' con el CSV".into()))?;

    if csv_content.trim().is_empty() {
        return Err(AppError::BadRequest("El archivo está vacío".into()));
    }

    let response = service::bulk_import_users(state.user_repo.as_ref(), &csv_content).await?;

    Ok(Json(response))
}

/// GET /users/me/notification-preferences
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user.id, user_email = %auth.user.email))]
pub async fn get_notification_preferences(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> ApiResult<Json<NotificationPreferences>> {
    let response =
        service::get_notification_preferences(state.user_repo.as_ref(), auth.user.id).await?;

    Ok(Json(response))
}

/// PATCH /users/me/notification-preferences
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user.id, user_email = %auth.user.email))]
pub async fn update_notification_preferences(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(payload): Json<UpdateNotificationPreferencesRequest>,
) -> ApiResult<Json<NotificationPreferences>> {
    let response =
        service::update_notification_preferences(state.user_repo.as_ref(), auth.user.id, payload)
            .await?;

    Ok(Json(response))
}
