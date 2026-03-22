# ytd-rs
[![Build status](https://github.com/nirusu99/ytd-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/nirusu99/ytd-rs/actions)
[![crates.io](https://img.shields.io/crates/v/ytd-rs.svg)](https://crates.io/crates/ytd-rs)
[![docs.rs](https://docs.rs/ytd-rs/badge.svg)](https://docs.rs/ytd-rs)

An async, feature-rich Rust wrapper for [yt-dlp](https://github.com/yt-dlp/yt-dlp).

## Features
- **Async API**: Built on top of `tokio`.
- **Builder Pattern**: Fluent API for configuring downloads.
- **Structured Metadata**: Extract video info as JSON via `serde`.
- **Custom Arguments**: Passthrough for any `yt-dlp` flag.

## Prerequisites
You must have `yt-dlp` installed and available in your `PATH`.

## Usage

### Simple Download
```rust
use ytd_rs::YtDlp;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ytd = YtDlp::new("https://www.youtube.com/watch?v=uTO0KnDsVH0")
        .output_dir(PathBuf::from("./downloads"))
        .arg("--quiet");

    let result = ytd.download().await?;
    println!("Output: {}", result.output());
    Ok(())
}
```

### Extract Audio
```rust
use ytd_rs::YtDlp;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    YtDlp::new("https://www.youtube.com/watch?v=uTO0KnDsVH0")
        .extract_audio(true)
        .audio_format("mp3")
        .download()
        .await?;
    Ok(())
}
```

### Get Video Metadata
```rust
use ytd_rs::YtDlp;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ytd = YtDlp::new("https://www.youtube.com/watch?v=uTO0KnDsVH0");
    let infos = ytd.get_info().await?;
    
    for info in infos {
        println!("Title: {}", info.title);
        println!("Duration: {:?}", info.duration);
    }
    Ok(())
}
```

### Streaming Output (Progress Bars)
If you want to track the progress of a long-running download, you can stream the standard output line-by-line:
```rust
use ytd_rs::YtDlp;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut process = YtDlp::new("https://www.youtube.com/watch?v=uTO0KnDsVH0")
        .download_process()
        .await?;

    while let Some(line) = process.next_line().await? {
        // e.g. "[download]  10.0% of 50.00MiB at 1.00MiB/s ETA 00:50"
        println!("yt-dlp output: {}", line);
    }

    process.wait().await?;
    println!("Download finished successfully!");
    Ok(())
}
```

## License
MIT
