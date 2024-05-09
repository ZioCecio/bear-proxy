use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct LoginDTO {
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
}
