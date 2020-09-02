pub mod frame;
pub mod request;
pub mod response;

use std::convert::TryFrom;
use std::net::Shutdown;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;

use frame::{Frame, Message};

#[derive(Debug)]
pub struct Client {
    pub framed: Framed,
    pub dispatcher: Sender<Client>,
}

impl Client {
    pub fn new(stream: TcpStream, dispatcher: Sender<Client>) -> Self {
        Self {
            framed: Framed { stream },
            dispatcher,
        }
    }

    pub fn shutdown(self) {
        let _ = self.framed.stream.shutdown(Shutdown::Both);
    }

    pub async fn dispatch(self) {
        let _ = self.dispatcher.clone().send(self).await;
    }

    pub async fn recv(&mut self) -> Result<Message, request::Error> {
        self.framed.recv().await
    }

    pub async fn send(&mut self, item: Message) -> Result<(), response::Error> {
        self.framed.send(item).await
    }
}

#[derive(Debug)]
pub struct Framed {
    pub stream: TcpStream,
}

impl Framed {
    pub async fn recv(&mut self) -> Result<Message, request::Error> {
        let header = self.stream.read_u16().await?;
        let mut bytes = vec![0u8; header as usize];
        self.stream.read_exact(&mut bytes[..]).await?;
        let str = std::str::from_utf8(&bytes[..])?;
        let frame = serde_json::from_str::<Frame>(&str)?;
        Ok(frame.message)
    }

    pub async fn send(&mut self, item: Message) -> Result<(), response::Error> {
        let frame = Frame { message: item };
        let string = serde_json::to_string(&frame)?;
        let header = u16::try_from(string.len())?;
        self.stream.write_u16(header).await?;
        self.stream.write_all(string.as_bytes()).await?;
        Ok(())
    }
}
