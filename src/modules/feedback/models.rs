use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SubmitFeedbackRequest {
    #[validate(length(
        min = 1,
        max = 200,
        message = "El nombre debe tener entre 1 y 200 caracteres"
    ))]
    pub name: String,

    #[validate(email(message = "El email no es válido"))]
    pub email: String,

    #[validate(length(max = 200, message = "El asunto no puede superar los 200 caracteres"))]
    pub subject: Option<String>,

    #[validate(length(
        min = 1,
        max = 5000,
        message = "El mensaje debe tener entre 1 y 5000 caracteres"
    ))]
    pub body: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitFeedbackResponse {
    pub message: String,
}
