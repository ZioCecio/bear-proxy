use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::sync::Mutex;

extern crate tokio;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct ProxyConfig {
    pub services: Vec<ServiceInfo>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct ServiceInfo {
    pub service_name: String,
    pub from: String,
    pub to: String,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let file_content = fs::read_to_string("config.yml")?;

    let proxy_config: ProxyConfig = serde_yaml::from_str(&file_content).unwrap();

    let mut tasks = Vec::new();
    let services_rules_buffer: Arc<Mutex<HashMap<String, Arc<Mutex<Vec<&[u8]>>>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    for service in proxy_config.services {
        let rules_buffer: Arc<Mutex<Vec<&[u8]>>> = Arc::new(Mutex::new(Vec::new()));
        services_rules_buffer
            .lock()
            .await
            .insert(service.service_name.clone(), rules_buffer.clone());

        tasks.push(start_service(service, rules_buffer.clone()));
    }

    futures::future::join_all(tasks).await;
    Ok(())
}

async fn start_service(service: ServiceInfo, rules_buffer: Arc<Mutex<Vec<&[u8]>>>) -> io::Result<()> {
    let from = &service.from;
    let listener = TcpListener::bind(from).await?;
    println!("Started service {}", service.service_name);

    loop {
        let to = service.to.clone();
        let (mut socket, addr) = listener.accept().await?;

        let rules_buffer = rules_buffer.clone();
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

            tokio::select! {
                _ = copy_with_abort(&mut remote_read, &mut client_write, cancel.subscribe(), false) => {},
                _ = copy_with_abort(&mut client_read, &mut remote_write, cancel.subscribe(), true) => {},
            };

            //cancel.send(()).unwrap();
        });
    }
}

const BUF_SIZE: usize = 1024;
async fn copy_with_abort<R, W>(
    read: &mut R,
    write: &mut W,
    mut abort: broadcast::Receiver<()>,
    is_client: bool,
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
            //for rule in rules

            if find_subsequence(&buf[0..bytes_read], b".git").is_some() {
                return Ok(0);
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
