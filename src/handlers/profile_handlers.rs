use crate::{
    model::{ProfileModel, UserModel},
    response::{AppError, AppPath, JsendResponse},
    AppState,
};
use axum::{
    extract::{Multipart, State},
    response::IntoResponse,
    Extension, Json,
};
use serde_json::json;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

pub async fn get_profile(
    AppPath(username): AppPath<String>,
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    // Query to find the user by username
    let user_id: Option<uuid::Uuid> =
        sqlx::query_scalar!("SELECT id FROM users WHERE username = $1", username)
            .fetch_optional(&data.db)
            .await
            .map_err(|_| AppError::InternalServerError)?;

    // If user is not found, return 404 Not Found
    let user_id = match user_id {
        Some(id) => id,
        _ => {
            return Err(AppError::JsendError("User not found".to_string()));
        }
    };

    // Query to find the profile by user_id
    let profile: Option<ProfileModel> = sqlx::query_as!(
        ProfileModel,
        "SELECT id, user_id, profile_image, bio, created_at, updated_at FROM profiles WHERE user_id = $1",
        user_id
    )
    .fetch_optional(&data.db)
    .await
    .map_err(|_| AppError::InternalServerError)?;

    // If profile is not found, return 404 Not Found
    let profile = match profile {
        Some(profile) => profile,
        None => {
            return Err(AppError::JsendFail(
                json!({"profile" : "profile not found"}),
            ));
        }
    };

    let response = JsendResponse::success(Some(json!({"profile" : profile})));
    Ok(Json(response))
}

pub async fn upload_profile_pic(
    Extension(user): Extension<UserModel>,
    State(data): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::InternalServerError)?
    {
        let content_type = match field.content_type() {
            Some(content) => content.to_string(),
            None => return Err(AppError::JsendError("invalid content-type".into())),
        };

        let file_extension = match content_type.as_str() {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            _ => return Err(AppError::JsendError("Unsupported file type".into())),
        };

        let bdata = match field.bytes().await {
            Ok(bdata) => bdata,
            Err(_) => return Err(AppError::JsendError("data error".into())),
        };

        let uuid_string = match user.id {
            Some(id) => id.to_string(),
            None => return Err(AppError::InternalServerError),
        };

        let assets_dir = PathBuf::from("./assets");

        let file_name = format!("{}.{}", uuid_string, file_extension);

        let file_path = assets_dir.join(&file_name);

        let mut file = File::create(file_path).map_err(|_| AppError::JsendError("DATA".into()))?;
        file.write_all(&bdata)
            .map_err(|_| AppError::InternalServerError)?;

        sqlx::query!(
            "UPDATE profiles SET profile_image = $1 WHERE user_id = $2",
            file_name,
            user.id
        )
        .execute(&data.db)
        .await
        .map_err(|_| AppError::InternalServerError)?;
    }

    let response = JsendResponse::success(None);
    Ok(Json(response))
}
