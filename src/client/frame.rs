use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub message: Message,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum Message {
    Connection(Connection),
    Session(Session),
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum Connection {
    End,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum Session {
    End,
    Request,
    Success,
    Value(String),
}
