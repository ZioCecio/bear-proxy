use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

extern crate tokio;

#[tokio::main]
async fn main() -> io::Result<()> {
    let from = "127.0.0.1:3000";
    let to = "127.0.0.1:8000";

    let listener = TcpListener::bind(from).await?;

    loop {
        let (mut socket, addr) = listener.accept().await?;
        tokio::spawn(async move {
            println!("New connection from {:?}!", addr);

            let mut remote = match TcpStream::connect(to).await {
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
            let request = std::str::from_utf8(&buf[0..bytes_read]).unwrap();
            if request.contains(".git") {
                return Ok(0);
            }
            // Other filters...

            println!("{:?}", request);
        }
        write.write_all(&buf[0..bytes_read]).await?;
        copied += bytes_read;
    }

    Ok(copied)
}
