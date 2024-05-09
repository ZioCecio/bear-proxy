use std::sync::Arc;

use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde_json::json;

use crate::models::rule::{ParsedRule, Rule, RuleAction, RuleDTO, RuleTypeDTO};
use crate::models::server::WebServerState;
use crate::utils::response::{get_json_response, get_response};

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

pub async fn get_rules_by_service_name(
    State(state): State<Arc<WebServerState>>,
    Path(service_name): Path<String>,
) -> Json<serde_json::Value> {
    let query = "
        SELECT * FROM rules
        WHERE service_name = ?1
    ";

    let connection = state.db_connection.lock().await;
    let mut statement = connection.prepare(query).unwrap();
    let mut rules = vec![];

    let rules_iter = statement
        .query_map([service_name], |row| {
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
    let bytes: Vec<u8>;
    if payload.rule_type == RuleTypeDTO::Ascii {
        bytes = payload.rule_text.as_bytes().to_vec();
    } else if payload.rule_type == RuleTypeDTO::Hex {
        match hex::decode(payload.rule_text) {
            Err(_) => {
                return get_response(StatusCode::BAD_REQUEST, "Invalid hex string.")
                    .into_response();
            }
            Ok(b) => bytes = b,
        }
    } else if payload.rule_type == RuleTypeDTO::Base64 {
        match BASE64_STANDARD.decode(&payload.rule_text) {
            Err(_) => {
                return get_response(StatusCode::BAD_REQUEST, "Invalid base64 string.")
                    .into_response();
            }
            Ok(b) => bytes = b,
        }
    } else {
        return get_response(StatusCode::BAD_REQUEST, "Invalid rule type.").into_response();
    }

    //let bytes: Vec<u8> = BASE64_STANDARD.decode(&payload.b64_rule).unwrap();
    let query = "
        INSERT INTO rules(rule, service_name)
        VALUES (?1, ?2)
    ";

    if !state.channels.contains_key(&payload.service_name) {
        return get_response(StatusCode::NOT_FOUND, "Service not found").into_response();
    }

    let b64_encoded_bytes = BASE64_STANDARD.encode(&bytes);
    let connection = state.db_connection.lock().await;
    connection
        //.execute(query, (payload.b64_rule, &payload.service_name))
        .execute(query, (&b64_encoded_bytes, &payload.service_name))
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

    let created_rule = Rule {
        id: connection.last_insert_rowid(),
        b64_rule: b64_encoded_bytes,
        service_name: payload.service_name,
    };

    get_json_response(StatusCode::CREATED, created_rule).into_response()
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
        return get_response(StatusCode::NOT_FOUND, "Rule not found.").into_response();
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

    get_response(StatusCode::OK, "Ok.").into_response()
}

pub async fn get_services_names(
    State(state): State<Arc<WebServerState>>,
) -> Json<serde_json::Value> {
    let channels = state.channels.clone();
    let mut services: Vec<String> = vec![];

    for service_name in channels.into_keys() {
        services.push(service_name);
    }

    Json(json!(services))
}
