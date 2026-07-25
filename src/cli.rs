use std::path::PathBuf;

use clap::{Parser, Subcommand, Args};

#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    pub title: Option<Vec<String>>,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Fix(Fix),
}

#[derive(Args)]
#[command(about = "Fixes given file or all files in the given folder")]
pub struct Fix {
    #[arg(group = "input")]
    pub title: Option<Vec<String>>,

    #[arg(long, short, group = "input")]
    pub path: Option<PathBuf>,
}
