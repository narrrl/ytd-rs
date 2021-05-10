//! Rust-Wrapper for youtube-dl

use std::{
    fs::{canonicalize, create_dir_all},
    io::Error,
    path::PathBuf,
    process::ExitStatus,
};
use std::{path::Path, process::Command};

pub struct YoutubeDL {
    path: PathBuf,
    link: String,
    args: Vec<(String, Option<String>)>,
}

impl YoutubeDL {
    pub fn new(
        dl_path: &str,
        args: Vec<(String, Option<String>)>,
        link: String,
    ) -> Result<YoutubeDL, String> {
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

        for (opt, arg) in self.args.iter() {
            match arg {
                Some(arg) => cmd.arg(opt).arg(arg),
                None => cmd.arg(opt),
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
