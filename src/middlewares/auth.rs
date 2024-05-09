use std::collections::BTreeMap;

use axum::{
    body::Body, extract::Request, http::StatusCode, middleware::Next, response::IntoResponse,
    Extension,
};
use axum_extra::extract::cookie::CookieJar;
use hmac::{Hmac, Mac};
use jwt::VerifyWithKey;
use sha2::Sha256;

use crate::utils::response::get_response;

pub async fn protect_api(
    Extension(sub): Extension<Option<String>>,
    request: Request,
    next: Next,
) -> impl IntoResponse {
    if let Some(_sub) = sub {
        return next.run(request).await;
    }

    get_response(StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
}

pub async fn extract_token(
    jar: CookieJar,
    mut request: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    let mut extracted_claims: Option<String> = None;

    if let Some(jwt) = jar.get("authToken") {
        let jwt = jwt.value();
        let jwt_secret =
            dotenv::var("JWT_SECRET").expect("Environment variable JWT_SECRET not set.");
        let key: Hmac<Sha256> = Hmac::new_from_slice(jwt_secret.as_bytes()).unwrap();

        let claims_result: Result<BTreeMap<String, String>, jwt::Error> = jwt.verify_with_key(&key);
        match claims_result {
            Ok(claims) => {
                extracted_claims = Some(claims["sub"].clone());
            }
            Err(_) => {}
        }
    }

    request.extensions_mut().insert(extracted_claims);

    next.run(request).await
}
