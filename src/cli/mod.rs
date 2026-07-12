// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod cli_impl;
mod decrypt;
mod generate;
mod validate;

pub use cli_impl::run_cli;

#[derive(Parser, Debug)]
#[command(name = "inklog")]
#[command(author = "Kirky.X")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "inklog - Enterprise-grade Rust logging infrastructure CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(name = "decrypt")]
    #[command(about = "Decrypt encrypted log files")]
    Decrypt {
        #[arg(short, long)]
        #[arg(help = "Input encrypted file or directory")]
        input: PathBuf,

        #[arg(short, long)]
        #[arg(help = "Output file or directory")]
        output: Option<PathBuf>,

        #[arg(short, long, env = "INKLOG_DECRYPT_KEY")]
        #[arg(help = "Environment variable name containing the decryption key")]
        key_env: String,

        #[arg(long)]
        #[arg(help = "Recursive decrypt directories")]
        recursive: bool,

        #[arg(long)]
        #[arg(help = "Batch mode: glob pattern for multiple files")]
        batch: bool,
    },

    #[command(name = "generate")]
    #[command(about = "Generate inklog configuration files")]
    Generate {
        #[arg(short, long)]
        #[arg(help = "Output directory or file path")]
        output: Option<PathBuf>,

        #[arg(short, long)]
        #[arg(help = "Config type: minimal, full, database, file")]
        #[arg(default_value = "full")]
        config_type: String,

        #[arg(long)]
        #[arg(help = "Generate environment variable example file")]
        env_example: bool,
    },

    #[command(name = "validate")]
    #[command(about = "Validate inklog configuration files")]
    Validate {
        #[arg(short, long)]
        #[arg(help = "Path to configuration file")]
        config: Option<PathBuf>,

        #[arg(long)]
        #[arg(help = "Check system prerequisites")]
        prerequisites: bool,
    },
}

fn main() {
    if let Err(e) = run_cli() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
