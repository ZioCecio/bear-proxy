pub mod controllers;
pub mod middlewares;
pub mod models;
pub mod utils;

use axum::middleware;
use axum::routing::delete;
use axum::routing::get;
use axum::routing::post;
use axum::Router;
use controllers::auth::get_token;
use controllers::pages::get_home_page;
use controllers::rules::add_rule;
use controllers::rules::delete_rule;
use controllers::rules::get_all_rules;
use controllers::rules::get_rules_by_service_name;
use controllers::rules::get_services_names;
use futures::future::BoxFuture;
use middlewares::auth::extract_token;
use middlewares::auth::protect_api;
use models::rule::ParsedRule;
use models::server::WebServerState;
use models::service::ProxyConfig;
use models::service::ServiceInfo;
use rusqlite::Connection;
use serde_yaml;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

use crate::models::rule::RuleAction;

extern crate tokio;

#[tokio::main]
async fn main() -> io::Result<()> {
    dotenv::dotenv().ok();
    let file_content = fs::read_to_string("config.yml")?;

    let proxy_config: ProxyConfig = serde_yaml::from_str(&file_content).unwrap();

    let mut channels: HashMap<String, Sender<ParsedRule>> = HashMap::new();
    let mut tasks: Vec<BoxFuture<_>> = Vec::new();
    for service in proxy_config.services {
        let (tx, rx): (Sender<ParsedRule>, Receiver<ParsedRule>) = mpsc::channel(1024);
        channels.insert(service.service_name.clone(), tx);

        tasks.push(Box::pin(start_service(service, rx)));
    }
    tasks.push(Box::pin(handle_rules(channels)));

    futures::future::join_all(tasks).await;
    Ok(())
}

async fn handle_rules(channels: HashMap<String, Sender<ParsedRule>>) -> io::Result<()> {
    let connection = Connection::open_in_memory().unwrap();
    let query = "
        CREATE TABLE rules(id INTEGER PRIMARY KEY, rule TEXT, service_name TEXT)
    ";
    connection.execute(query, ()).unwrap();

    let shared_state = Arc::new(WebServerState {
        channels,
        db_connection: Arc::new(Mutex::new(connection)),
    });

    let app = Router::new()
        .route("/rules", get(get_all_rules))
        .route(
            "/rules/filter/:service_name",
            get(get_rules_by_service_name),
        )
        .route("/services", get(get_services_names))
        .route("/rules", post(add_rule))
        .route("/rules/:rule_id", delete(delete_rule))
        .route_layer(middleware::from_fn(protect_api))
        .route("/front", get(get_home_page))
        .route_layer(middleware::from_fn(extract_token))
        .route("/get_token", post(get_token))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:1234").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn start_service(service: ServiceInfo, mut rx: Receiver<ParsedRule>) -> io::Result<()> {
    let from = &service.from;
    let listener = TcpListener::bind(from).await?;
    println!("Started service {}", service.service_name);

    let rules: Arc<Mutex<HashMap<usize, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
    loop {
        let rules_clone = Arc::clone(&rules);

        let to = service.to.clone();
        let (mut socket, addr) = listener.accept().await?;

        loop {
            match rx.try_recv() {
                Ok(message) => {
                    println!("Thread {:?}: {:?}", service.service_name, message.rule);

                    match message.action {
                        RuleAction::AddRule => {
                            rules.lock().await.insert(message.id, message.rule.unwrap());
                        }
                        RuleAction::RemoveRule => {
                            rules.lock().await.remove(&message.id);
                        }
                    }
                }
                Err(_) => break,
            }
        }

        tokio::spawn(async move {
            println!("New connection from {:?}!", addr);

            let mut remote = match TcpStream::connect(&to).await {
                Ok(socket) => socket,
                Err(e) => {
                    eprintln!("Error estabilishing connection to the server: {e}");
                    return;
                }
            };

            let (mut client_read, mut client_write) = socket.split();
            let (mut remote_read, mut remote_write) = remote.split();

            let (cancel, _) = broadcast::channel::<()>(1);

            let rules_reference = rules_clone.lock().await.clone();
            tokio::select! {
                _ = copy_with_abort(&mut remote_read, &mut client_write, cancel.subscribe(), false, None) => {},
                _ = copy_with_abort(&mut client_read, &mut remote_write, cancel.subscribe(), true, Some(rules_reference)) => {},
            };
        });
    }
}

const BUF_SIZE: usize = 1024;
async fn copy_with_abort<R, W>(
    read: &mut R,
    write: &mut W,
    mut abort: broadcast::Receiver<()>,
    is_client: bool,
    rules: Option<HashMap<usize, Vec<u8>>>,
) -> tokio::io::Result<usize>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut copied = 0;
    let mut buf = [0u8; BUF_SIZE];
    loop {
        let bytes_read;
        tokio::select! {
            biased;

            result = read.read(&mut buf) => {
                use std::io::ErrorKind::{ConnectionReset, ConnectionAborted};
                bytes_read = result.or_else(|e| match e.kind() {
                    ConnectionReset | ConnectionAborted => Ok(0),
                    _ => Err(e)
                })?;
            },
            _ = abort.recv() => {
                break;
            }
        }

        if bytes_read == 0 {
            break;
        }

        if is_client {
            for (_, rule) in rules.as_ref().unwrap() {
                if find_subsequence(&buf[0..bytes_read], &rule).is_some() {
                    return Ok(0);
                }
            }
            // Other filters...
        }

        write.write_all(&buf[0..bytes_read]).await?;
        copied += bytes_read;
    }

    Ok(copied)
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
