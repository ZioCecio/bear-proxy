use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

use crate::models::server::ServerResponse;

pub fn get_response(status_code: StatusCode, message: &str) -> impl IntoResponse {
    return (
        status_code,
        Json(ServerResponse {
            message: message.to_string(),
        }),
    );
}

pub fn get_json_response<T: Serialize>(status_code: StatusCode, json: T) -> impl IntoResponse {
    return (status_code, Json(json));
}
