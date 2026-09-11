// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use super::query::QueryArgs;
use super::{Cli, Commands, decrypt, generate, query, validate};

pub fn run_cli() -> i32 {
    let args = Cli::parse();
    let json = args.json;
    match run_with_args(args) {
        Ok(code) => code,
        Err(e) => {
            if json {
                eprintln!(
                    "{}",
                    serde_json::json!({ "status": "error", "message": e.to_string() })
                );
            } else {
                eprintln!("Error: {}", e);
            }
            1
        }
    }
}

/// 退出码契约（T505/T510）：`0` = 成功/有匹配，`1` = 错误，`2` = 无匹配。
pub(crate) fn run_with_args(args: Cli) -> Result<i32> {
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
                let input_str = input.to_str().ok_or_else(|| {
                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set("path", format!("{:?}", input));
                    anyhow::anyhow!(
                        "{}",
                        inklog::i18n::tr_args("cli-decrypt-err-input-utf8", args)
                    )
                })?;
                decrypt::batch_decrypt(input_str, &output, &key_env)?;
            } else if input.is_file() {
                decrypt::decrypt_file_compatible(&input, &output, &key_env)?;
            } else {
                decrypt::decrypt_directory_compatible(&input, &output, &key_env, recursive)?;
            }
            if args.json {
                // T510：机器可读结果（人类可读的 i18n 消息仅文本模式输出）
                println!(
                    "{}",
                    serde_json::json!({
                        "command": "decrypt",
                        "status": "ok",
                        "input": input.display().to_string(),
                        "output": output.display().to_string(),
                    })
                );
            } else {
                let mut fluent = fluent_bundle::FluentArgs::new();
                fluent.set("input", input.display().to_string());
                fluent.set("output", output.display().to_string());
                println!(
                    "{}",
                    if input.is_dir() {
                        inklog::i18n::tr_args("cli-decrypt-dir-done", fluent)
                    } else {
                        inklog::i18n::tr_args("cli-decrypt-done", fluent)
                    }
                );
            }
            Ok(0)
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

            generate::generate_config(&output_path, &config_type.to_string())?;

            if env_example {
                generate::generate_env_example(&output_path)?;
            }
            if args.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "command": "generate",
                        "status": "ok",
                        "output": output_path.display().to_string(),
                        "config_type": config_type.to_string(),
                        "env_example": env_example,
                    })
                );
            }
            Ok(0)
        }

        Commands::Validate {
            config,
            prerequisites,
        } => {
            if prerequisites {
                if args.json {
                    validate::set_quiet_output(true);
                }
                let result = validate::check_prerequisites();
                validate::set_quiet_output(false);
                result?;
                if args.json {
                    println!(
                        "{}",
                        serde_json::json!({ "command": "validate", "target": "prerequisites", "status": "ok" })
                    );
                }
                return Ok(0);
            }

            let config_path = config.unwrap_or_else(|| PathBuf::from("inklog_config.toml"));
            if args.json {
                validate::set_quiet_output(true);
            }
            let result = validate::validate_config(&config_path);
            validate::set_quiet_output(false);
            match result {
                Ok(()) => {
                    if args.json {
                        println!(
                            "{}",
                            serde_json::json!({
                                "command": "validate",
                                "config": config_path.display().to_string(),
                                "status": "ok",
                            })
                        );
                    }
                    Ok(0)
                }
                Err(e) => {
                    if args.json {
                        println!(
                            "{}",
                            serde_json::json!({
                                "command": "validate",
                                "config": config_path.display().to_string(),
                                "status": "invalid",
                                "message": e.to_string(),
                            })
                        );
                        // 校验失败属"发现问题"而非执行错误 → 退出码 1
                        Ok(1)
                    } else {
                        Err(e)
                    }
                }
            }
        }

        Commands::Query {
            path,
            since,
            until,
            level,
            grep,
            limit,
            key_env,
        } => {
            let qargs = QueryArgs {
                path,
                since,
                until,
                level,
                grep,
                limit,
                key_env,
                json: args.json,
            };
            query::run_query(&qargs)
        }
    }
}
