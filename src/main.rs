use std::{
    io::{self, Write},
};

mod constants;
mod cli;
pub mod components;
pub mod downloader;
pub mod lyrics;
pub mod parser;
pub mod song;
mod yt_dlp;
mod yt_dlp_cli;

pub use constants::*;
use clap::Parser;
use filess::Ogg;
pub use yt_dlp::*;
pub use yt_dlp_cli::*;

use crate::{cli::Commands, song::Song};

pub const VERSION: &str = "0.2.0";

pub fn log_step(message: &str) {
    // \r moves cursor to start, \x1b[K clears the rest of the line
    print!("\r\x1b[K[ STATUS ] {}", message);
    io::stdout().flush().unwrap();
}

#[tokio::main]
async fn main() {
    let cli = cli::Cli::parse();

    match &cli.command {
        Some(Commands::Fix(fix)) => {
            let path = if let Some(title) = &fix.title {
                dirs::audio_dir().unwrap().join(&title.join(""))
            } else if let Some(path) = &fix.path {
                path.clone()
            } else {
                dirs::audio_dir().unwrap()
            };

            let files: Vec<Ogg> = if path.is_dir() {
                filess::walkdir::WalkDir::new(&path)
                    .into_iter()
                    .filter_map(|entry| entry.ok()) // Ignore permission / access errors
                    .filter(|entry| entry.file_type().is_file()) // Ensure it's a file, not a directory
                    .filter(|entry| {
                        entry
                            .path()
                            .extension()
                            .and_then(std::ffi::OsStr::to_str)
                            .map_or(false, |ext| ext.eq_ignore_ascii_case("opus"))
                    })
                    .map(|entry| Ogg::new(entry.into_path()))
                    .collect()
            } else {
                vec![Ogg::new(path)]
            };

            for file in files {
                let old = file.to_str().unwrap();
                println!("Fixing {}", &old);
                let song = Song::fix(&file);
                let new = song.file.to_str().unwrap();
                if old != new {
                    println!("Fixed {old} => {new}", );
                }
            }
        }
        None => {},
    }

    if let Some(title) = cli.title.as_deref() {
        let title = title.join(" ");
        let d = downloader::Downloader {};
        d.download(title).await;
    } else if cli.command.is_none() {
        let d = downloader::Downloader {};
        d.download(downloader::Downloader::prompt_input(
            "Enter the song name or URL:",
            "e.g. Never Gonna Give You Up",
        ))
        .await;
    }
}
