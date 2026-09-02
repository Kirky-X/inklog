// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use clap::{Parser, Subcommand, ValueEnum};
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
        #[arg(value_parser = clap::builder::NonEmptyStringValueParser::new())]
        key_env: String,

        #[arg(long)]
        #[arg(help = "Recursive decrypt directories")]
        recursive: bool,

        #[arg(long)]
        #[arg(help = "Enable batch mode for multiple files")]
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
        config_type: ConfigType,

        #[arg(long)]
        #[arg(help = "Generate environment variable example file")]
        env_example: bool,
    },

    #[command(name = "validate")]
    #[command(about = "Validate inklog configuration files (default: inklog_config.toml)")]
    Validate {
        #[arg(short, long)]
        #[arg(help = "Path to configuration file")]
        config: Option<PathBuf>,

        #[arg(long)]
        #[arg(help = "Check system prerequisites instead of config file")]
        prerequisites: bool,
    },
}

/// Configuration template type for the generate command.
#[derive(Debug, Clone, ValueEnum)]
pub enum ConfigType {
    Minimal,
    Full,
    Database,
    File,
}

impl std::fmt::Display for ConfigType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigType::Minimal => write!(f, "minimal"),
            ConfigType::Full => write!(f, "full"),
            ConfigType::Database => write!(f, "database"),
            ConfigType::File => write!(f, "file"),
        }
    }
}

fn main() {
    if let Err(e) = run_cli() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

// 测试进程固定 locale 为 en：与 CI（Linux，LANG 未设）行为一致。
// bin 测试链接的是非 test 编译的 lib，lib 内的 cfg(test) 分支在此
// 不生效；ctor 于 main() 之前设置文档化最高优先级 override
// （INKLOG_LOCALE，见 i18n/locale_manager.rs），对全进程测试生效。
#[cfg(test)]
#[ctor::ctor(unsafe)]
fn init_test_locale() {
    // 测试进程启动期（单线程、无其他线程读 env），set_var 无 UB 风险
    unsafe {
        std::env::set_var("INKLOG_LOCALE", "en");
    }
}
