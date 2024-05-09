use std::{collections::BTreeMap, env};

use axum::{http::StatusCode, response::IntoResponse, Json};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use sha2::Sha256;

use crate::models::{auth::LoginDTO, server::ServerResponse};

pub async fn get_token(jar: CookieJar, Json(login_dto): Json<LoginDTO>) -> impl IntoResponse {
    let env_password = env::var("AUTH_PASSWORD").expect("Variable AUTH_PASSWORD not set.");

    if env_password != login_dto.password {
        return (
            StatusCode::UNAUTHORIZED,
            jar,
            Json(ServerResponse {
                message: "Wrong password".to_string(),
            }),
        );
    }

    let jwt_secret = dotenv::var("JWT_SECRET").expect("Environment variable JWT_SECRET not set.");
    let key: Hmac<Sha256> = Hmac::new_from_slice(jwt_secret.as_bytes()).unwrap();
    let mut claims = BTreeMap::new();
    claims.insert("sub", "ziocecio");

    if let Ok(token_str) = claims.sign_with_key(&key) {
        return (
            StatusCode::OK,
            jar.add(Cookie::new("authToken", token_str)),
            Json(ServerResponse {
                message: "Ok".to_string(),
            }),
        );
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        jar,
        Json(ServerResponse {
            message: "Internal server error".to_string(),
        }),
    )
}
