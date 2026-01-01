use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod decrypt;
mod generate;
mod validate;

#[derive(Parser, Debug)]
#[command(name = "inklog")]
#[command(author = "Kirky.X")]
#[command(version = "0.1.0")]
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

pub fn run_cli() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Decrypt {
            input,
            output,
            key_env,
            recursive,
            batch,
        } => {
            let output = output.unwrap_or_else(|| {
                if input.is_file() {
                    input.with_extension("decrypted.log")
                } else {
                    input.join("decrypted")
                }
            });

            if batch {
                decrypt::batch_decrypt(input.to_str().unwrap_or("*"), &output, &key_env)?;
            } else if input.is_file() {
                decrypt::decrypt_file_compatible(&input, &output, &key_env)?;
                println!("Decrypted: {} -> {}", input.display(), output.display());
            } else {
                decrypt::decrypt_directory_compatible(&input, &output, &key_env, recursive)?;
                println!(
                    "Decrypted all files in {} to {}",
                    input.display(),
                    output.display()
                );
            }
        }

        Commands::Generate {
            output,
            config_type,
            env_example,
        } => {
            let output_path = output.unwrap_or_else(|| PathBuf::from("."));
            let output_path = if output_path.is_dir() {
                output_path
            } else {
                output_path
                    .parent()
                    .unwrap_or(&PathBuf::from("."))
                    .to_path_buf()
            };

            generate::generate_config(&output_path, &config_type)?;

            if env_example {
                generate::generate_env_example(&output_path)?;
            }
        }

        Commands::Validate {
            config,
            prerequisites,
        } => {
            if prerequisites {
                validate::check_prerequisites();
                return Ok(());
            }

            let config_path = config.unwrap_or_else(|| PathBuf::from("inklog_config.toml"));
            validate::validate_config(&config_path)?;
        }
    }

    Ok(())
}
