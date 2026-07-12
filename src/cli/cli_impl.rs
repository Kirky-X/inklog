// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use super::{Cli, Commands};
use super::{decrypt, generate, validate};

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
