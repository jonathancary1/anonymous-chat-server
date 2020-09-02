use futures::future::{BoxFuture, FutureExt};
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::mpsc;

use crate::client::frame::{Connection, Message, Session};
use crate::client::Client;

pub async fn listen<A: ToSocketAddrs>(addr: A) {
    let mut listener = TcpListener::bind(addr).await.unwrap();
    let (sender, receiver) = mpsc::channel::<Client>(1);
    tokio::spawn(dispatcher(receiver));
    loop {
        if let Ok((stream, addr)) = listener.accept().await {
            println!("Accepted connection from {}.", addr);
            tokio::spawn(receive_session_request(Client::new(stream, sender.clone())));
        }
    }
}

async fn dispatcher(mut receiver: mpsc::Receiver<Client>) {
    while let Some(mut left) = receiver.recv().await {
        tokio::select! {
            result = left.recv() => {
                match result {
                    Ok(Message::Session(Session::End)) => send_session_end(left).await,
                    _ => left.shutdown(),
                }
            },
            Some(right) = receiver.recv() => {
                tokio::spawn(send_session_success((left, right)));
            },
            else => {
                break;
            },
        };
    }
    eprintln!("All Sender halves have dropped.");
}

fn receive_session_request(mut client: Client) -> BoxFuture<'static, ()> {
    async move {
        match client.recv().await {
            Ok(Message::Connection(Connection::End)) => send_connection_end(client).await,
            Ok(Message::Session(Session::End)) => receive_session_request(client).await,
            Ok(Message::Session(Session::Request)) => client.dispatch().await,
            Ok(Message::Session(Session::Value(_))) => receive_session_request(client).await,
            _ => client.shutdown(),
        }
    }
    .boxed()
}

fn receive_session_value(mut clients: (Client, Client)) -> BoxFuture<'static, ()> {
    async move {
        tokio::select! {
            result = clients.0.recv() => {
                match result {
                    Ok(Message::Session(Session::Value(value))) => {
                        send_session_value((clients.0, clients.1), value).await;
                    },
                    Ok(Message::Session(Session::End)) => {
                        tokio::join!(send_session_end(clients.0), send_session_end(clients.1));
                    },
                    _ => {
                        clients.0.shutdown();
                        send_session_end(clients.1).await;
                    },
                }
            },
            result = clients.1.recv() => {
                match result {
                    Ok(Message::Session(Session::Value(value))) => {
                        send_session_value((clients.1, clients.0), value).await;
                    },
                    Ok(Message::Session(Session::End)) => {
                        tokio::join!(send_session_end(clients.0), send_session_end(clients.1));
                    },
                    _ => {
                        clients.1.shutdown();
                        send_session_end(clients.0).await;
                    },
                }
            }
        };
    }
    .boxed()
}

async fn send_connection_end(mut client: Client) {
    let _ = client.send(Message::Connection(Connection::End)).await;
    client.shutdown();
}

async fn send_session_end(mut client: Client) {
    match client.send(Message::Session(Session::End)).await {
        Ok(()) => {
            tokio::spawn(receive_session_request(client));
        }
        Err(_) => client.shutdown(),
    }
}

async fn send_session_success(mut clients: (Client, Client)) {
    let result = tokio::join!(
        clients.0.send(Message::Session(Session::Success)),
        clients.1.send(Message::Session(Session::Success))
    );

    if let (Ok(()), Ok(())) = result {
        receive_session_value(clients).await;
    } else {
        if let Err(_) = result.0 {
            clients.0.shutdown();
        } else {
            send_session_end(clients.0).await;
        }
        if let Err(_) = result.1 {
            clients.1.shutdown();
        } else {
            send_session_end(clients.1).await;
        }
    }
}

async fn send_session_value(mut clients: (Client, Client), value: String) {
    let message = Message::Session(Session::Value(value));
    match clients.1.send(message).await {
        Ok(()) => {
            receive_session_value(clients).await;
        }
        Err(_) => {
            clients.1.shutdown();
            send_session_end(clients.0).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::net::{TcpStream, ToSocketAddrs};
    use tokio::runtime::Runtime;

    use crate::client::frame::{Connection, Message, Session};
    use crate::client::Framed;
    use crate::protocol;

    #[test]
    fn test_connection_end() {
        Runtime::new().unwrap().block_on(async {
            tokio::spawn(protocol::listen("::1:3000"));
            connection_end("::1:3000").await;
        });
    }

    async fn connection_end<A: ToSocketAddrs>(addr: A) {
        let stream = TcpStream::connect(addr).await.unwrap();
        let mut framed = Framed { stream };
        let connection_end = Message::Connection(Connection::End);
        framed.send(connection_end.clone()).await.unwrap();
        assert_eq!(framed.recv().await.unwrap(), connection_end);
    }

    #[test]
    fn test_session_end() {
        Runtime::new().unwrap().block_on(async {
            tokio::spawn(protocol::listen("::1:3001"));
            let a = session_end("::1:3001");
            let b = session_end("::1:3001");
            tokio::join!(a, b);
        });
    }

    async fn session_end<A: ToSocketAddrs>(addr: A) {
        let stream = TcpStream::connect(addr).await.unwrap();
        let mut framed = Framed { stream };
        let session_request = Message::Session(Session::Request);
        let session_success = Message::Session(Session::Success);
        let session_end = Message::Session(Session::End);
        framed.send(session_request).await.unwrap();
        assert_eq!(framed.recv().await.unwrap(), session_success);
        framed.send(session_end.clone()).await.unwrap();
        assert_eq!(framed.recv().await.unwrap(), session_end);
    }

    #[test]
    fn test_session_success() {
        Runtime::new().unwrap().block_on(async {
            tokio::spawn(protocol::listen("::1:3002"));
            let a = session_success("::1:3002");
            let b = session_success("::1:3002");
            tokio::join!(a, b);
        });
    }

    async fn session_success<A: ToSocketAddrs>(addr: A) {
        let stream = TcpStream::connect(addr).await.unwrap();
        let mut framed = Framed { stream };
        let session_request = Message::Session(Session::Request);
        let session_success = Message::Session(Session::Success);
        framed.send(session_request).await.unwrap();
        assert_eq!(framed.recv().await.unwrap(), session_success);
    }

    #[test]
    fn test_session_value() {
        Runtime::new().unwrap().block_on(async {
            tokio::spawn(protocol::listen("::1:3003"));
            let a = session_value("::1:3003", "A".to_string(), "B".to_string());
            let b = session_value("::1:3003", "B".to_string(), "A".to_string());
            tokio::join!(a, b);
        });
    }

    async fn session_value<A: ToSocketAddrs>(addr: A, send: String, recv: String) {
        let stream = TcpStream::connect(addr).await.unwrap();
        let mut framed = Framed { stream };
        let session_request = Message::Session(Session::Request);
        let session_success = Message::Session(Session::Success);
        let session_value_send = Message::Session(Session::Value(send));
        let session_value_recv = Message::Session(Session::Value(recv));
        framed.send(session_request).await.unwrap();
        assert_eq!(framed.recv().await.unwrap(), session_success);
        framed.send(session_value_send).await.unwrap();
        assert_eq!(framed.recv().await.unwrap(), session_value_recv);
    }

    #[test]
    fn test_ignore_session_end() {
        Runtime::new().unwrap().block_on(async {
            tokio::spawn(protocol::listen("::1:3004"));
            ignore_session_end("::1:3004").await;
        });
    }

    async fn ignore_session_end<A: ToSocketAddrs>(addr: A) {
        let stream = TcpStream::connect(addr).await.unwrap();
        let mut framed = Framed { stream };
        let session_end = Message::Session(Session::End);
        let connection_end = Message::Connection(Connection::End);
        framed.send(session_end).await.unwrap();
        framed.send(connection_end.clone()).await.unwrap();
        assert_eq!(framed.recv().await.unwrap(), connection_end);
    }

    #[test]
    fn test_ignore_session_value() {
        Runtime::new().unwrap().block_on(async {
            tokio::spawn(protocol::listen("::1:3005"));
            ignore_session_value("::1:3005").await;
        });
    }

    async fn ignore_session_value<A: ToSocketAddrs>(addr: A) {
        let stream = TcpStream::connect(addr).await.unwrap();
        let mut framed = Framed { stream };
        let session_value = Message::Session(Session::Value("".to_string()));
        let connection_end = Message::Connection(Connection::End);
        framed.send(session_value).await.unwrap();
        framed.send(connection_end.clone()).await.unwrap();
        assert_eq!(framed.recv().await.unwrap(), connection_end);
    }

    #[test]
    fn test_invalid_request() {
        Runtime::new().unwrap().block_on(async {
            tokio::spawn(protocol::listen("::1:3006"));
            invalid_request("::1:3006").await;
        });
    }

    async fn invalid_request<A: ToSocketAddrs>(addr: A) {
        let stream = TcpStream::connect(addr).await.unwrap();
        let mut framed = Framed { stream };
        let session_request = Message::Session(Session::Request);
        framed.send(session_request.clone()).await.unwrap();
        framed.send(session_request).await.unwrap();
        assert!(framed.recv().await.is_err());
    }
}
