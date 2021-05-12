//! Rust-Wrapper for youtube-dl
//!
//! # Example
//!
//! ```no_run
//! use ytd_rs::{YoutubeDL, ResultType, Arg};
//! // youtube-dl arguments quietly run process and to format the output
//! // one doesn't take any input and is an option, the other takes the desired output format as input
//! let args = vec![Arg::new("--quiet"), Arg::new_with_arg("--output", "%(title).90s.%(ext)s")];
//! let link = "https://www.youtube.com/watch?v=uTO0KnDsVH0";
//! let ytd = YoutubeDL::new("./path/to/download/directory", args, link).unwrap();
//!
//! // start download
//! let download = ytd.download();
//!
//! // check what the result is and print out the path to the download or the error
//! match download.result_type() {
//!     ResultType::SUCCESS => println!("Your download: {}", download.output_dir().to_string_lossy()),
//!     ResultType::IOERROR | ResultType::FAILURE =>
//!             println!("Couldn't start download: {}", download.output()),
//! };
//! ```

use std::{
    fmt,
    process::{Output, Stdio},
};
use std::{
    fmt::{Display, Formatter},
    fs::{canonicalize, create_dir_all},
    io::Error,
    path::PathBuf,
};
use std::{path::Path, process::Command};

/// A structure that represents an argument of a youtube-dl command.
///
/// There are two different kinds of Arg:
/// - Option with no other input
/// - Argument with input
///
/// # Example
///
/// ```
/// use ytd_rs::Arg;
/// // youtube-dl option to embed metadata into the file
/// // doesn't take any input
/// let simple_arg = Arg::new("--add-metadata");
///
/// // youtube-dl cookies argument that takes a path to
/// // cookie file
/// let input_arg = Arg::new_with_arg("--cookie", "/path/to/cookie");
/// ```
#[derive(Clone, Debug)]
pub struct Arg {
    arg: String,
    input: Option<String>,
}

impl Arg {
    pub fn new(argument: &str) -> Arg {
        Arg {
            arg: argument.to_string(),
            input: None,
        }
    }

    pub fn new_with_arg(argument: &str, input: &str) -> Arg {
        Arg {
            arg: argument.to_string(),
            input: Option::from(input.to_string()),
        }
    }
}

impl Display for Arg {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        match &self.input {
            Some(input) => write!(fmt, "{} {}", self.arg, input),
            None => write!(fmt, "{}", self.arg),
        }
    }
}

/// Structure that represents a youtube-dl task.
///
/// Every task needs a download location, a list of ['Arg'] that can be empty
/// and a ['link'] to the desired source.
#[derive(Clone, Debug)]
pub struct YoutubeDL {
    path: PathBuf,
    links: Vec<String>,
    args: Vec<Arg>,
}

///
/// This is the result of a [`YoutubeDL`].
///
/// It contains the information about the exit status, the output and the directory it was executed
/// in.
///
#[derive(Debug, Clone)]
pub struct YoutubeDLResult {
    path: PathBuf,
    output: String,
    kind: ResultType,
}

impl YoutubeDLResult {
    /// creates a new YoutubeDLResult
    fn new(path: PathBuf) -> YoutubeDLResult {
        YoutubeDLResult {
            path,
            output: String::new(),
            kind: ResultType::FAILURE,
        }
    }

    /// get the exit status of the process
    pub fn result_type(&self) -> &ResultType {
        &self.kind
    }

    /// get the output of the youtube-dl process
    pub fn output(&self) -> &str {
        &self.output
    }

    /// get the directory where youtube-dl was executed
    pub fn output_dir(&self) -> &PathBuf {
        &self.path
    }
}

/// Represents the different exit types of the [`YoutubeDL`] process
#[derive(Debug, Clone)]
pub enum ResultType {
    SUCCESS,
    FAILURE,
    IOERROR,
}

impl YoutubeDL {
    /// Creates a new YoutubeDL job to be executed.
    /// It takes a path where youtube-dl should be executed, a vec! of [`Arg`] that can be empty
    /// and finally a link that can be `""` if no video should be downloaded
    ///
    /// The path gets canonicalized and the directory gets created by the constructor
    pub fn new_multiple_links(
        dl_path: &str,
        args: Vec<Arg>,
        links: Vec<String>,
    ) -> Result<YoutubeDL, String> {
        // create path
        let path = Path::new(dl_path);

        // check if it already exists
        if !path.exists() {
            // if not create
            if let Err(why) = create_dir_all(&path) {
                return Err(format!(
                    "Error while creating directories {}: {:?}",
                    dl_path, why
                ));
            }
        }

        // return error if no directory
        if !path.is_dir() {
            return Err("Error: path is not a directory".to_string());
        }

        // absolute path
        match canonicalize(dl_path) {
            // return new youtube-dl job
            Ok(path) => Ok(YoutubeDL { path, links, args }),
            Err(why) => Err(format!("Error creating YouTubeDL: {:?}", why)),
        }
    }

    pub fn new(dl_path: &str, args: Vec<Arg>, link: &str) -> Result<YoutubeDL, String> {
        YoutubeDL::new_multiple_links(dl_path, args, vec![link.to_string()])
    }

    /// Starts the download and returns when finished the result as [`YoutubeDLResult`].
    pub fn download(&self) -> YoutubeDLResult {
        let pr_result = self.spawn_youtube_dl();
        let mut result = YoutubeDLResult::new(self.path.clone());

        let output = match pr_result {
            Err(why) => {
                result.output = why.to_string();
                result.kind = ResultType::IOERROR;
                return result;
            }
            Ok(output) => output,
        };

        if output.status.success() {
            result.kind = ResultType::SUCCESS;
            result.output = String::from_utf8_lossy(&output.stdout).to_string();
        } else {
            result.output = String::from_utf8_lossy(&output.stderr).to_string();
        }

        result
    }

    fn spawn_youtube_dl(&self) -> Result<Output, Error> {
        let mut cmd = Command::new("youtube-dl");
        cmd.current_dir(&self.path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for arg in self.args.iter() {
            match &arg.input {
                Some(input) => cmd.arg(&arg.arg).arg(input),
                None => cmd.arg(&arg.arg),
            };
        }

        for link in self.links.iter() {
            cmd.arg(&link);
        }

        let pr = match cmd.spawn() {
            Err(why) => {
                return Err(why);
            }
            Ok(process) => process,
        };
        pr.wait_with_output()
    }
}

#[cfg(test)]
mod tests {
    use crate::{Arg, YoutubeDL};
    use regex::Regex;
    use std::env;

    #[test]
    fn version() {
        let ytd = YoutubeDL::new(
            env::current_dir()
                .expect("couldn't get working directory")
                .to_str()
                .unwrap(),
            // get youtube-dl version
            vec![Arg::new("--version")],
            // we don't need a link to print version
            "",
        )
        .expect("Couldn't create youtube-dl");

        let regex = Regex::new(r"\d{4}\.\d{2}\.\d{2}").unwrap();
        let output = ytd.download();

        // check output
        // fails if youtube-dl is not installed
        assert!(regex.is_match(output.output()));
    }
}
