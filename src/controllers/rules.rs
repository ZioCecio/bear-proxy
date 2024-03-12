use std::sync::Arc;

use axum::{extract::State, Json};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde_json::json;

use crate::models::rule::RuleDTO;
use crate::models::server::WebServerState;

pub async fn get_all_rules(State(state): State<Arc<WebServerState>>) -> Json<serde_json::Value> {
    let query = "
        SELECT * FROM rules
    ";

    let connection = state.db_connection.lock().await;
    let rules = connection.execute(query).unwrap();

    Json(json!(rules))
}

pub async fn add_rule(
    State(state): State<Arc<WebServerState>>,
    Json(payload): Json<RuleDTO>,
) -> &'static str {
    let _bytes: Vec<u8> = BASE64_STANDARD.decode(&payload.b64_rule).unwrap();
    let query = "
        INSERT INTO rules(id, rule)
        VALUES (:id, :rule)
    ";

    let connection = state.db_connection.lock().await;
    let mut statement = connection.prepare(query).unwrap();
    statement
        .bind_iter::<_, (_, sqlite::Value)>([
            (":id", 0.into()),
            (":rule", payload.b64_rule.as_str().into()),
        ])
        .unwrap();

    "Ok"
}
