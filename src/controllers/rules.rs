use std::sync::Arc;

use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde_json::json;

use crate::models::rule::{ParsedRule, Rule, RuleAction, RuleDTO};
use crate::models::server::{ServerResponse, WebServerState};

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
                service_name: row.get(2).unwrap(),
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
) -> impl IntoResponse {
    let bytes: Vec<u8> = BASE64_STANDARD.decode(&payload.b64_rule).unwrap();
    let query = "
        INSERT INTO rules(rule, service_name)
        VALUES (?1, ?2)
    ";

    if !state.channels.contains_key(&payload.service_name) {
        return (
            StatusCode::NOT_FOUND,
            Json(ServerResponse {
                message: "Service not found".to_string(),
            }),
        )
            .into_response();
    }

    let connection = state.db_connection.lock().await;
    connection
        .execute(query, (payload.b64_rule, &payload.service_name))
        .unwrap();
    let parsed_rule = ParsedRule {
        id: connection.last_insert_rowid() as usize,
        service_name: Some(payload.service_name.clone()),
        rule: Some(bytes),
        action: RuleAction::AddRule,
    };
    state
        .channels
        .get(&payload.service_name)
        .unwrap()
        .send(parsed_rule)
        .await
        .unwrap();

    (
        StatusCode::OK,
        Json(ServerResponse {
            message: "Ok".to_string(),
        }),
    )
        .into_response()
}

pub async fn delete_rule(
    State(state): State<Arc<WebServerState>>,
    Path(rule_id): Path<usize>,
) -> impl IntoResponse {
    let select_query = "
        SELECT service_name FROM rules
        WHERE id = ?1
    ";

    let query = "
        DELETE FROM rules WHERE id = ?1
    ";

    let connection = state.db_connection.lock().await;

    let service_name: String = match connection.query_row(select_query, [rule_id], |row| row.get(0))
    {
        Ok(name) => name,
        Err(_) => "".to_string(),
    };

    connection.execute(query, (rule_id,)).unwrap();
    let deleted = connection.changes();
    if deleted == 0 {
        return (
            StatusCode::NOT_FOUND,
            Json(ServerResponse {
                message: "Rule not found".to_string(),
            }),
        )
            .into_response();
    }

    let parsed_rule = ParsedRule {
        id: connection.last_insert_rowid() as usize,
        service_name: None,
        rule: None,
        action: RuleAction::RemoveRule,
    };

    state
        .channels
        .get(&service_name)
        .unwrap()
        .send(parsed_rule)
        .await
        .unwrap();

    (
        StatusCode::OK,
        Json(ServerResponse {
            message: "Ok".to_string(),
        }),
    )
        .into_response()
}
