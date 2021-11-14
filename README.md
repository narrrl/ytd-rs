# ytd-rs
[![Build status](https://github.com/nirusu99/ytd-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/nirusu99/ytd-rs/actions)
[![crates.io](https://img.shields.io/crates/v/ytd-rs.svg)](https://crates.io/crates/ytd-rs)
[![docs.rs](https://docs.rs/ytd-rs/badge.svg)](https://docs.rs/ytd-rs)

This is a simple wrapper for [youtube-dl](https://youtube-dl.org/) in rust.

```rust
use ytd_rs::{YoutubeDL, Arg};
use std::path::PathBuf;
use std::error::Error;
fn main() -> Result<(), Box<dyn Error>> {
    // youtube-dl arguments quietly run process and to format the output
    // one doesn't take any input and is an option, the other takes the desired output format as input
    let args = vec![Arg::new("--quiet"), Arg::new_with_arg("--output", "%(title).90s.%(ext)s")];
    let link = "https://www.youtube.com/watch?v=uTO0KnDsVH0";
    let path = PathBuf::from("./path/to/download/directory");
    let ytd = YoutubeDL::new(&path, args, link)?;

    // start download
    let download = ytd.download()?;

    // print out the download path
    println!("Your download: {}", download.output_dir().to_string_lossy())
    Ok(())
}
```
