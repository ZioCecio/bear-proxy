pub mod controllers;
pub mod  models;

use axum::routing::get;
use axum::routing::post;
use axum::Router;
use controllers::rules::add_rule;
use controllers::rules::get_all_rules;
use futures::future::BoxFuture;
use models::rule::ParsedRule;
use models::server::WebServerState;
use models::service::ProxyConfig;
use models::service::ServiceInfo;
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

extern crate tokio;


#[tokio::main]
async fn main() -> io::Result<()> {
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

//async fn handle_rules(channels: HashMap<String, Sender<ParsedRule>>) -> io::Result<()> {
//    let from = "127.0.0.1:1234";
//    let listener = TcpListener::bind(from).await?;
//    println!("Started rule service");
//
//    loop {
//        let (mut socket, addr) = listener.accept().await?;
//        println!("New connection from {:?}!", addr);
//        let (mut client_read, mut client_write) = socket.split();
//
//        const BUFFER_SIZE: usize = 1024;
//        let mut bytes_to_add: Vec<u8> = vec![];
//        let mut buf = [0u8; BUFFER_SIZE];
//        let mut read_bytes = BUFFER_SIZE;
//        let mut cycles = 0;
//
//        while read_bytes == BUFFER_SIZE {
//            read_bytes = client_read.read(&mut buf).await.unwrap();
//            bytes_to_add.append(&mut buf.to_vec());
//            cycles += 1;
//        }
//
//        match parse_rule(&bytes_to_add[..(BUFFER_SIZE * (cycles - 1)) + read_bytes - 1], 0) {
//            Some(rule_info) => match channels.get(&rule_info.service_name) {
//                Some(channel) => {
//                    let send_result = channel.send(rule_info).await;
//                    if send_result.is_ok() {
//                        client_write.write("Rule added!".as_bytes()).await.unwrap();
//                    } else {
//                        client_write
//                            .write("Rule not added...".as_bytes())
//                            .await
//                            .unwrap();
//                    }
//                }
//                None => {
//                    client_write
//                        .write("Invalid service name!".as_bytes())
//                        .await
//                        .unwrap();
//                }
//            },
//            None => {
//                client_write
//                    .write("Invalid rule format!".as_bytes())
//                    .await
//                    .unwrap();
//            }
//        }
//    }
//}

async fn handle_rules(channels: HashMap<String, Sender<ParsedRule>>) -> io::Result<()> {
    
    let connection = sqlite::open(":memory:").unwrap();
    let query = "
        CREATE TABLE rules(id INTEGER, rule TEXT)
    ";
    connection.execute(query).unwrap();

    let shared_state = Arc::new(WebServerState { channels, db_connection: Arc::new(Mutex::new(connection)) });

    let app = Router::new()
        .route("/rules", get(get_all_rules))
        .route("/rules", post(add_rule))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:1234").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    
    Ok(())
}

async fn start_service(service: ServiceInfo, mut rx: Receiver<ParsedRule>) -> io::Result<()> {
    let from = &service.from;
    let listener = TcpListener::bind(from).await?;
    println!("Started service {}", service.service_name);

    let rules: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
    loop {
        let rules_clone = Arc::clone(&rules);

        let to = service.to.clone();
        let (mut socket, addr) = listener.accept().await?;

        loop {
            match rx.try_recv() {
                Ok(message) => {
                    println!("Thread {:?}: {:?}", service.service_name, message.rule);
                    rules.lock().await.push(message.rule);
                }
                Err(_) => break
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
    rules: Option<Vec<Vec<u8>>>,
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
            for rule in rules.as_ref().unwrap() {
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
