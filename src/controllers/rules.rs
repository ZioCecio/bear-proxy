use std::sync::Arc;

use axum::{extract::State, Json};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde_json::json;

use crate::models::rule::{ParsedRule, Rule, RuleAction, RuleDTO};
use crate::models::server::WebServerState;

pub async fn get_all_rules(State(state): State<Arc<WebServerState>>) -> Json<serde_json::Value> {
    let query = "
        SELECT * FROM rules
    ";

    let connection = state.db_connection.lock().await;
    let mut statement = connection.prepare(query).unwrap();
    let mut rules = vec![];

    let rules_iter = statement
        .query_map([], |row| {
            Ok(Rule {
                id: row.get(0).unwrap(),
                b64_rule: row.get(1).unwrap(),
            })
        })
        .unwrap();

    for rule in rules_iter {
        rules.push(rule.unwrap());
    }

    Json(json!(rules))
}

pub async fn add_rule(
    State(state): State<Arc<WebServerState>>,
    Json(payload): Json<RuleDTO>,
) -> &'static str {
    let bytes: Vec<u8> = BASE64_STANDARD.decode(&payload.b64_rule).unwrap();
    let query = "
        INSERT INTO rules(id, rule)
        VALUES (?1, ?2)
    ";

    if !state.channels.contains_key(&payload.service_name) {
        return "Error";
    }

    let connection = state.db_connection.lock().await;
    connection.execute(query, (0, payload.b64_rule)).unwrap();
    let parsed_rule = ParsedRule {
        id: 0,
        service_name: payload.service_name.clone(),
        rule: bytes,
        action: RuleAction::AddRule,
    };
    state
        .channels
        .get(&payload.service_name)
        .unwrap()
        .send(parsed_rule)
        .await
        .unwrap();

    "Ok"
}
