use thiserror::Error;

#[derive(Error, Debug)]
pub enum YtDlpError {
    #[error("failed to execute yt-dlp: {0}")]
    IOError(#[from] std::io::Error),
    #[error("failed to convert output to string: {0}")]
    UTF8Error(#[from] std::string::FromUtf8Error),
    #[error("failed to parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("yt-dlp exited with status {code:?}: {stderr}")]
    Failure { code: Option<i32>, stderr: String },
}
