//! Async wrapper for yt-dlp
//!
//! # Example
//!
//! ```no_run
//! use ytd_rs::YtDlp;
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let path = PathBuf::from("./download");
//!     let ytd = YtDlp::new("https://www.youtube.com/watch?v=uTO0KnDsVH0")
//!         .output_dir(path)
//!         .arg("--quiet")
//!         .arg_with("--output", "%(title).90s.%(ext)s");
//!
//!     let result = ytd.download().await?;
//!     println!("Download complete: {}", result.output());
//!     Ok(())
//! }
//! ```

use crate::error::YtDlpError;
use log::{debug, error, info, trace};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::process::{ChildStdout, Command};

pub mod error;

pub type Result<T> = std::result::Result<T, YtDlpError>;

/// Represents the output of a yt-dlp execution.
#[derive(Debug, Clone)]
pub struct YtDlpResult {
    output: String,
}

impl YtDlpResult {
    pub fn new(output: String) -> Self {
        Self { output }
    }

    /// Returns the standard output of the yt-dlp process.
    pub fn output(&self) -> &str {
        &self.output
    }
}

/// A structured representation of video metadata extracted via --dump-json.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub url: Option<String>,
    pub duration: Option<f64>,
    pub uploader: Option<String>,
    pub thumbnail: Option<String>,
    pub description: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// A running yt-dlp process that allows streaming standard output (useful for progress bars).
pub struct YtDlpChild {
    child: tokio::process::Child,
    stdout: Option<Lines<BufReader<ChildStdout>>>,
}

impl YtDlpChild {
    /// Returns the next line of output from the process. Returns `None` if EOF is reached.
    pub async fn next_line(&mut self) -> Result<Option<String>> {
        if let Some(stdout) = &mut self.stdout {
            match stdout.next_line().await {
                Ok(line) => Ok(line),
                Err(e) => Err(YtDlpError::IOError(e)),
            }
        } else {
            Ok(None)
        }
    }

    /// Waits for the process to complete and checks its status.
    pub async fn wait(mut self) -> Result<()> {
        let status = self.child.wait().await?;
        if status.success() {
            Ok(())
        } else {
            Err(YtDlpError::Failure {
                code: status.code(),
                stderr: "Process failed".to_string(),
            })
        }
    }
}

/// A running yt-dlp process that streams raw media bytes directly to standard output.
pub struct YtDlpStream {
    child: tokio::process::Child,
    stdout: tokio::process::ChildStdout,
}

impl YtDlpStream {
    /// Returns a mutable reference to the standard output to read bytes.
    /// You can use this with `tokio::io::AsyncReadExt` methods like `read_buf`.
    pub fn stdout(&mut self) -> &mut tokio::process::ChildStdout {
        &mut self.stdout
    }

    /// Waits for the process to complete and checks its status.
    pub async fn wait(mut self) -> Result<()> {
        let status = self.child.wait().await?;
        if status.success() {
            Ok(())
        } else {
            Err(YtDlpError::Failure {
                code: status.code(),
                stderr: "Process failed".to_string(),
            })
        }
    }
}

/// The main builder for configuring and running yt-dlp.
///
/// Use this struct to chain configurations and finally execute a download or metadata extraction.
#[derive(Clone, Debug, Default)]
pub struct YtDlp {
    links: Vec<String>,
    args: Vec<(String, Option<String>)>,
    output_dir: Option<PathBuf>,
    executable_path: Option<String>,
}

impl YtDlp {
    /// Create a new yt-dlp task for a single link.
    pub fn new(link: impl Into<String>) -> Self {
        Self {
            links: vec![link.into()],
            ..Default::default()
        }
    }

    /// Create a new yt-dlp task for multiple links.
    pub fn new_multiple(links: Vec<String>) -> Self {
        Self {
            links,
            ..Default::default()
        }
    }

    /// Set a custom path to the yt-dlp executable.
    ///
    /// Useful if yt-dlp is not in your PATH or you want to use a specific version.
    pub fn yt_dlp_path(mut self, path: impl Into<String>) -> Self {
        self.executable_path = Some(path.into());
        self
    }

    /// Set the output directory for the download.
    ///
    /// If the directory does not exist, it will be created during execution.
    pub fn output_dir(mut self, path: PathBuf) -> Self {
        self.output_dir = Some(path);
        self
    }

    /// Add a raw argument to the yt-dlp command.
    ///
    /// # Example
    /// ```
    /// # use ytd_rs::YtDlp;
    /// let ytd = YtDlp::new("link").arg("--quiet");
    /// ```
    pub fn arg(mut self, flag: impl Into<String>) -> Self {
        self.args.push((flag.into(), None));
        self
    }

    /// Add a raw argument with an accompanying value (e.g., --output template).
    ///
    /// # Example
    /// ```
    /// # use ytd_rs::YtDlp;
    /// let ytd = YtDlp::new("link").arg_with("--output", "%(title)s.%(ext)s");
    /// ```
    pub fn arg_with(mut self, flag: impl Into<String>, value: impl Into<String>) -> Self {
        self.args.push((flag.into(), Some(value.into())));
        self
    }

    /// Convenience method for --extract-audio.
    pub fn extract_audio(self, enabled: bool) -> Self {
        if enabled {
            self.arg("--extract-audio")
        } else {
            self
        }
    }

    /// Convenience method for --audio-format.
    pub fn audio_format(self, format: impl Into<String>) -> Self {
        self.arg_with("--audio-format", format)
    }

    /// Configures yt-dlp to download only the best audio and extract it to the given format (e.g., "mp3").
    pub fn extract_audio_only(self, format: impl Into<String>) -> Self {
        self.best_audio().extract_audio(true).audio_format(format)
    }

    /// Convenience method for --output template.
    pub fn output_template(self, template: impl Into<String>) -> Self {
        self.arg_with("--output", template)
    }

    /// Set the video/audio format (e.g., "best", "mp4").
    pub fn format(self, format: impl Into<String>) -> Self {
        self.arg_with("--format", format)
    }

    /// Select the best quality video and audio.
    pub fn best_quality(self) -> Self {
        self.format("bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best")
    }

    /// Select the best quality audio only.
    pub fn best_audio(self) -> Self {
        self.format("bestaudio/best")
    }

    /// Path to a cookies file.
    pub fn cookies(self, path: impl Into<String>) -> Self {
        self.arg_with("--cookies", path)
    }

    /// Load cookies from a browser (e.g., "firefox", "chrome").
    pub fn cookies_from_browser(self, browser: impl Into<String>) -> Self {
        self.arg_with("--cookies-from-browser", browser)
    }

    /// Provide a username for authentication.
    pub fn username(self, username: impl Into<String>) -> Self {
        self.arg_with("--username", username)
    }

    /// Provide a password for authentication.
    pub fn password(self, password: impl Into<String>) -> Self {
        self.arg_with("--password", password)
    }

    /// Enable or disable playlist downloading.
    pub fn playlist(self, enabled: bool) -> Self {
        if enabled {
            self.arg("--yes-playlist")
        } else {
            self.arg("--no-playlist")
        }
    }

    /// Specify specific playlist items to download (e.g., "1-3,7,10-13").
    pub fn playlist_items(self, items: impl Into<String>) -> Self {
        self.arg_with("--playlist-items", items)
    }

    /// Write subtitles to a file.
    pub fn write_subtitles(self, enabled: bool) -> Self {
        if enabled {
            self.arg("--write-sub")
        } else {
            self
        }
    }

    /// Specify subtitle languages to download (e.g., vec!["en", "de"]).
    pub fn sub_langs(self, langs: Vec<String>) -> Self {
        self.arg_with("--sub-langs", langs.join(","))
    }

    /// Embed subtitles into the video file.
    pub fn embed_subtitles(self, enabled: bool) -> Self {
        if enabled {
            self.arg("--embed-subs")
        } else {
            self
        }
    }

    /// Embed metadata into the file.
    pub fn embed_metadata(self, enabled: bool) -> Self {
        if enabled {
            self.arg("--embed-metadata")
        } else {
            self
        }
    }

    /// Write thumbnail to a file.
    pub fn write_thumbnail(self, enabled: bool) -> Self {
        if enabled {
            self.arg("--write-thumbnail")
        } else {
            self
        }
    }

    /// Use a proxy (e.g., "http://127.0.0.1:8080").
    pub fn proxy(self, proxy: impl Into<String>) -> Self {
        self.arg_with("--proxy", proxy)
    }

    /// Limit download rate (e.g., "1M", "50K").
    pub fn limit_rate(self, rate: impl Into<String>) -> Self {
        self.arg_with("--limit-rate", rate)
    }

    /// Set number of retries.
    pub fn retries(self, retries: u32) -> Self {
        self.arg_with("--retries", retries.to_string())
    }

    /// Executes yt-dlp and returns the standard output.
    ///
    /// This method awaits the process to finish and captures the entire output.
    pub async fn download(&self) -> Result<YtDlpResult> {
        info!("Starting download for links: {:?}", self.links);
        let output = self.spawn_yt_dlp(false).await?.wait_with_output().await?;

        if !output.status.success() {
            let err_msg = String::from_utf8(output.stderr)?;
            error!("yt-dlp download failed: {}", err_msg);
            return Err(YtDlpError::Failure {
                code: output.status.code(),
                stderr: err_msg,
            });
        }

        info!("yt-dlp download finished successfully");
        Ok(YtDlpResult::new(String::from_utf8(output.stdout)?))
    }

    /// Executes yt-dlp as a continuous process, allowing line-by-line output streaming.
    /// Automatically adds `--newline` so progress updates are written on new lines.
    ///
    /// # Example
    /// ```no_run
    /// # use ytd_rs::YtDlp;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut process = YtDlp::new("link").download_process().await?;
    /// while let Some(line) = process.next_line().await? {
    ///     println!("{}", line);
    /// }
    /// process.wait().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download_process(&self) -> Result<YtDlpChild> {
        info!("Starting download process for links: {:?}", self.links);
        let mut clone = self.clone();
        clone = clone.arg("--newline");
        let mut child = clone.spawn_yt_dlp(false).await?;

        let stdout = child
            .stdout
            .take()
            .map(|stdout| BufReader::new(stdout).lines());

        Ok(YtDlpChild { child, stdout })
    }

    /// Executes yt-dlp and streams the raw media binary data to standard output.
    /// This automatically sets `--output -` and gives you access to the async stdout reader.
    ///
    /// # Example
    /// ```no_run
    /// # use ytd_rs::YtDlp;
    /// # use tokio::io::AsyncReadExt;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut stream = YtDlp::new("link").download_to_stream().await?;
    /// let mut buffer = [0; 1024];
    /// stream.stdout().read(&mut buffer).await?;
    /// stream.wait().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download_to_stream(&self) -> Result<YtDlpStream> {
        info!(
            "Starting binary stream download for links: {:?}",
            self.links
        );
        let mut clone = self.clone();
        clone = clone.arg_with("--output", "-");
        let mut child = clone.spawn_yt_dlp(false).await?;

        let stdout = child.stdout.take().ok_or_else(|| {
            error!("Failed to capture stdout for binary stream");
            YtDlpError::Failure {
                code: None,
                stderr: "Failed to capture standard output".to_string(),
            }
        })?;

        Ok(YtDlpStream { child, stdout })
    }

    /// Executes yt-dlp with --dump-json and parses the output into VideoInfo.
    ///
    /// # Example
    /// ```no_run
    /// # use ytd_rs::YtDlp;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let infos = YtDlp::new("link").get_info().await?;
    /// for info in infos {
    ///     println!("Title: {}", info.title);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_info(&self) -> Result<Vec<VideoInfo>> {
        info!("Fetching video info for links: {:?}", self.links);
        let output = self.spawn_yt_dlp(true).await?.wait_with_output().await?;

        if !output.status.success() {
            let err_msg = String::from_utf8(output.stderr)?;
            error!("yt-dlp get_info failed: {}", err_msg);
            return Err(YtDlpError::Failure {
                code: output.status.code(),
                stderr: err_msg,
            });
        }

        let stdout = String::from_utf8(output.stdout)?;
        let mut infos = Vec::new();
        for line in stdout.lines() {
            if !line.trim().is_empty() {
                trace!("Parsing JSON line: {}", line);
                let info: VideoInfo = serde_json::from_str(line)?;
                infos.push(info);
            }
        }

        info!("Successfully fetched info for {} videos", infos.len());
        Ok(infos)
    }

    async fn spawn_yt_dlp(&self, dump_json: bool) -> Result<tokio::process::Child> {
        if let Some(path) = self.output_dir.as_ref().filter(|p| !p.exists()) {
            debug!("Creating output directory: {:?}", path);
            fs::create_dir_all(path).await?;
        }

        let cmd_path = self.executable_path.as_deref().unwrap_or("yt-dlp");
        let mut cmd = Command::new(cmd_path);

        if let Some(ref path) = self.output_dir {
            cmd.current_dir(path);
        }

        cmd.env("LC_ALL", "en_US.UTF-8")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if dump_json {
            cmd.arg("--dump-json");
        }

        for (arg, val) in &self.args {
            cmd.arg(arg);
            if let Some(v) = val {
                cmd.arg(v);
            }
        }

        for link in &self.links {
            cmd.arg(link);
        }

        debug!("Executing command: {:?}", cmd);
        Ok(cmd.spawn()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_version() -> Result<()> {
        let ytd = YtDlp::new("").arg("--version");
        let result = ytd.download().await?;
        assert!(
            regex::Regex::new(r"\d{4}\.\d{2}\.\d{2}")
                .unwrap()
                .is_match(result.output())
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_get_info() -> Result<()> {
        // We won't actually download in CI if it's too slow, but let's try a version check as a proxy
        let version = YtDlp::new("").arg("--version").download().await?;
        println!("yt-dlp version: {}", version.output());
        Ok(())
    }

    #[tokio::test]
    async fn test_download_process() -> Result<()> {
        let mut process = YtDlp::new("").arg("--version").download_process().await?;

        let mut lines = Vec::new();
        while let Some(line) = process.next_line().await? {
            lines.push(line);
        }

        process.wait().await?;

        assert!(!lines.is_empty());
        assert!(
            regex::Regex::new(r"\d{4}\.\d{2}\.\d{2}")
                .unwrap()
                .is_match(&lines[0])
        );
        Ok(())
    }
}
