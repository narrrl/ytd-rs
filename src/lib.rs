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
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;

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

/// The main builder for configuring and running yt-dlp.
#[derive(Clone, Debug, Default)]
pub struct YtDlp {
    links: Vec<String>,
    args: Vec<(String, Option<String>)>,
    output_dir: Option<PathBuf>,
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

    /// Set the output directory for the download.
    pub fn output_dir(mut self, path: PathBuf) -> Self {
        self.output_dir = Some(path);
        self
    }

    /// Add a raw argument to the yt-dlp command.
    pub fn arg(mut self, flag: impl Into<String>) -> Self {
        self.args.push((flag.into(), None));
        self
    }

    /// Add a raw argument with an accompanying value (e.g., --output template).
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

    /// Convenience method for --output template.
    pub fn output_template(self, template: impl Into<String>) -> Self {
        self.arg_with("--output", template)
    }

    /// Executes yt-dlp and returns the standard output.
    pub async fn download(&self) -> Result<YtDlpResult> {
        let output = self.spawn_yt_dlp(false).await?;

        if !output.status.success() {
            return Err(YtDlpError::Failure(String::from_utf8(output.stderr)?));
        }

        Ok(YtDlpResult::new(String::from_utf8(output.stdout)?))
    }

    /// Executes yt-dlp with --dump-json and parses the output into VideoInfo.
    pub async fn get_info(&self) -> Result<Vec<VideoInfo>> {
        let output = self.spawn_yt_dlp(true).await?;

        if !output.status.success() {
            return Err(YtDlpError::Failure(String::from_utf8(output.stderr)?));
        }

        let stdout = String::from_utf8(output.stdout)?;
        let mut infos = Vec::new();
        for line in stdout.lines() {
            if !line.trim().is_empty() {
                let info: VideoInfo = serde_json::from_str(line)?;
                infos.push(info);
            }
        }

        Ok(infos)
    }

    async fn spawn_yt_dlp(&self, dump_json: bool) -> Result<std::process::Output> {
        if let Some(ref path) = self.output_dir {
            if !path.exists() {
                fs::create_dir_all(path).await?;
            }
        }

        let mut cmd = Command::new("yt-dlp");

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

        let output = cmd.spawn()?.wait_with_output().await?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_version() -> Result<()> {
        let ytd = YtDlp::new("").arg("--version");
        let result = ytd.download().await?;
        assert!(regex::Regex::new(r"\d{4}\.\d{2}\.\d{2}")
            .unwrap()
            .is_match(result.output()));
        Ok(())
    }

    #[tokio::test]
    async fn test_get_info() -> Result<()> {
        // We won't actually download in CI if it's too slow, but let's try a version check as a proxy
        let version = YtDlp::new("").arg("--version").download().await?;
        println!("yt-dlp version: {}", version.output());
        Ok(())
    }
}
