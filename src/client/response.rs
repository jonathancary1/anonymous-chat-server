#[derive(Debug)]
pub enum Error {
    IO(tokio::io::Error),
    Json(serde_json::Error),
    Overflow(std::num::TryFromIntError),
}

impl From<tokio::io::Error> for Error {
    fn from(error: tokio::io::Error) -> Self {
        Self::IO(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(error: std::num::TryFromIntError) -> Self {
        Self::Overflow(error)
    }
}
