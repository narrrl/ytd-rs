use thiserror::Error;

#[derive(Error, Debug)]
pub enum YoutubeDLError {
    #[error("failed to execute youtube-dl")]
    IOError(#[from] std::io::Error),
    #[error("failed to convert path")]
    UTF8Error(#[from] std::string::FromUtf8Error),
    #[error("youtube-dl exited with: {0}")]
    Failure(String),
}
