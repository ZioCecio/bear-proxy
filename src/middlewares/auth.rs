use axum::{extract::Request, http::StatusCode, middleware::Next, response::IntoResponse};
use axum_extra::extract::cookie::CookieJar;

use crate::utils::response::get_response;

pub async fn auth(jar: CookieJar, request: Request, next: Next) -> impl IntoResponse {
    if let Some(jwt) = jar.get("authToken") {

        let response = next.run(request).await;
        response
    } else {
        get_response(StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
    }
}