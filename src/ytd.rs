//! Rust-Wrapper for youtube-dl

use std::fmt;
use std::{
    fmt::{Display, Formatter},
    fs::{canonicalize, create_dir_all},
    io::Error,
    path::PathBuf,
    process::ExitStatus,
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
/// // youtube-dl option to embed metadata into the file
/// // doesn't take any input
/// let simple_arg = Arg::new("--add-metadata");
///
/// // youtube-dl cookies argument that takes a path to
/// // cookie file
/// let input_arg = Arg::new("--cookie", "/path/to/cookie");
/// ```
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
/// Every task needs a download location [`path`], a list of ['Arg'] that can be empty
/// and a ['link'] to the desired source.
pub struct YoutubeDL {
    path: PathBuf,
    link: String,
    args: Vec<Arg>,
}

impl YoutubeDL {
    pub fn new(dl_path: &str, args: Vec<Arg>, link: String) -> Result<YoutubeDL, String> {
        // create path
        let path = Path::new(dl_path);

        // check if it already exists
        if !path.exists() {
            // if not create
            if let Err(why) = create_dir_all(&path) {
                return Err(format!("Error: {:?}", why));
            }
        }

        // return error if no directory
        if !path.is_dir() {
            return Err("Error: path is not a directory".to_string());
        }

        // absolute path
        match canonicalize(dl_path) {
            // return new youtube-dl job
            Ok(path) => Ok(YoutubeDL { path, link, args }),
            Err(why) => Err(format!("Error: {:?}", why)),
        }
    }

    pub fn download(&self) -> Result<&PathBuf, String> {
        let result = self.spawn_youtube_dl();

        if let Err(why) = result {
            Err(format!("Error: {:?}", why))
        } else {
            Ok(&self.path)
        }
    }

    fn spawn_youtube_dl(&self) -> Result<ExitStatus, Error> {
        let mut path = self.path.clone();
        path.push("%(title).90s.%(ext)s");
        let mut cmd = Command::new("youtube-dl");
        cmd.arg("--output")
            .arg(format!("{}", path.display()))
            .arg("--quiet")
            .arg("--no-warnings");

        for arg in self.args.iter() {
            match &arg.input {
                Some(input) => cmd.arg(&arg.arg).arg(input),
                None => cmd.arg(&arg.arg),
            };
        }

        cmd.arg(&self.link);

        let mut pr = match cmd.spawn() {
            Err(why) => panic!("couldn't spawn youtube-dl: {:?}", why),
            Ok(process) => process,
        };
        pr.wait()
    }
}
