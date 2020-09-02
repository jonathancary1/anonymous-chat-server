#[derive(Debug)]
pub enum Error {
    IO(tokio::io::Error),
    Json(serde_json::Error),
    Utf8(std::str::Utf8Error),
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

impl From<std::str::Utf8Error> for Error {
    fn from(error: std::str::Utf8Error) -> Self {
        Self::Utf8(error)
    }
}
