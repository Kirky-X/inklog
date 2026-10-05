// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
use anyhow::{Context as _, Result};
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
                // 本地化输出：InklogError 走 FTL 前缀（localized_message 路径）；
                // 其余错误（CLI 层多为已本地化的 anyhow 消息）按 Display 原样输出
                let msg = e
                    .downcast_ref::<inklog::InklogError>()
                    .map(|err| err.localized_message())
                    .unwrap_or_else(|| e.to_string());
                eprintln!("{}: {}", inklog::i18n::tr("cli-err-prefix"), msg);
            }
            1
        }
    }
}

/// 退出码契约：`0` = 成功/有匹配，`1` = 错误，`2` = 无匹配。
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
                    let mut args = inklog::i18n::MsgArgs::new();
                    args.set("path", format!("{:?}", input));
                    anyhow::anyhow!(
                        "{}",
                        inklog::i18n::tr_args("cli-decrypt-err-input-utf8", args)
                    )
                })?;
                decrypt::batch_decrypt(input_str, &output, &key_env)?;
            } else if input.is_file() {
                decrypt::decrypt_file_compatible(&input, &output, &key_env)?;
            } else if !input.exists() {
                // 不存在的路径在分支判定里两个谓词都是 false，会静默滑进
                // 目录解密分支——显式拦截给出明确错误
                let mut args = inklog::i18n::MsgArgs::new();
                args.set("path", input.display().to_string());
                return Err(anyhow::anyhow!(
                    "{}",
                    inklog::i18n::tr_args("cli-decrypt-err-input-not-found", args)
                ));
            } else {
                decrypt::decrypt_directory_compatible(&input, &output, &key_env, recursive)?;
            }
            if args.json {
                // 机器可读结果（人类可读的 i18n 消息仅文本模式输出）
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
                let mut fluent = inklog::i18n::MsgArgs::new();
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
            schema,
        } => {
            // --schema：输出 InklogConfig 的 JSON Schema 文件（默认
            // config_schema.json，与入库产物同源）；--output 此时为文件路径
            if schema {
                // 本分支以 return 发散，output 可直接 move，不占用后续路径
                let schema_path = output.unwrap_or_else(|| PathBuf::from("config_schema.json"));
                generate::generate_config_schema(&schema_path)?;
                if args.json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "command": "generate",
                            "status": "ok",
                            "schema": true,
                            "output": schema_path.display().to_string(),
                        })
                    );
                }
                return Ok(0);
            }

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
                        let status = if config_path.exists() {
                            "invalid"
                        } else {
                            "error"
                        };
                        println!(
                            "{}",
                            serde_json::json!({
                                "command": "validate",
                                "config": config_path.display().to_string(),
                                "status": status,
                                "message": e.to_string(),
                            })
                        );
                        // validate 的失败（无效/错误）统一退出码 1
                        Ok(1)
                    } else {
                        Err(e)
                    }
                }
            }
        }

        Commands::VerifyChain { manifest, key_env } => verify_chain_manifest(&manifest, &key_env),
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

// ============================================================================
// 全子命令 --json + 稳定退出码（0/1/2）契约单测
// ============================================================================

/// `verify-chain`：重算 HMAC 链并核对 manifest 完整性。
///
/// 退出码契约：`0` = 链完整，`1` = 错误，`2` = 篡改/校验失败。
fn verify_chain_manifest(manifest: &std::path::Path, key_env: &str) -> Result<i32> {
    use inklog::support::audit_chain::ArchiveChainEntry;

    let data = std::fs::read_to_string(manifest)
        .with_context(|| inklog::i18n::tr("cli-verify-chain-err-manifest-unreadable"))?;
    let mut entries: Vec<ArchiveChainEntry> = Vec::new();
    for (lineno, line) in data.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let entry: ArchiveChainEntry = serde_json::from_str(line).map_err(|e| {
            anyhow::anyhow!(
                "{}:{}: {e}",
                inklog::i18n::tr("cli-verify-chain-err-line-invalid"),
                lineno + 1
            )
        })?;
        entries.push(entry);
    }
    let key = std::env::var(key_env)
        .with_context(|| inklog::i18n::tr("cli-verify-chain-err-key-missing"))?;

    let intact =
        inklog::support::audit_chain::ArchiveChain::verify_entries(&entries, key.as_bytes());
    if intact {
        let mut args = inklog::i18n::MsgArgs::new();
        args.set("chain", inklog::i18n::tr("cli-verify-chain-ok"));
        args.set("path", manifest.display().to_string());
        args.set("count", entries.len());
        println!(
            "{}",
            inklog::i18n::tr_args("cli-verify-chain-ok-detail", args)
        );
    } else {
        println!(
            "{}: {}",
            inklog::i18n::tr("cli-verify-chain-tampered"),
            manifest.display()
        );
    }
    Ok(if intact { 0 } else { 2 })
}

#[cfg(test)]
mod exit_code_tests {
    use super::*;
    use std::fs;

    fn cli(json: bool, command: Commands) -> Cli {
        Cli { json, command }
    }

    fn tempdir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn test_validate_valid_config_exit_0() {
        let dir = tempdir();
        let cfg = dir.path().join("valid.toml");
        fs::write(&cfg, "[global]\nlevel = \"info\"\n").unwrap();
        let code = run_with_args(cli(
            false,
            Commands::Validate {
                config: Some(cfg),
                prerequisites: false,
            },
        ))
        .unwrap();
        assert_eq!(code, 0, "valid config must exit 0");
    }

    #[test]
    fn test_validate_invalid_config_json_exit_1() {
        let dir = tempdir();
        let cfg = dir.path().join("invalid.toml");
        fs::write(&cfg, "[global]\nlevel = \"not-a-level\"\n").unwrap();
        let code = run_with_args(cli(
            true,
            Commands::Validate {
                config: Some(cfg),
                prerequisites: false,
            },
        ))
        .unwrap();
        assert_eq!(
            code, 1,
            "invalid config must exit 1 (found problem, not an error)"
        );
    }

    #[test]
    fn test_validate_missing_config_json_status_error_exit_1() {
        let code = run_with_args(cli(
            true,
            Commands::Validate {
                config: Some(PathBuf::from("/nonexistent/inklog/config.toml")),
                prerequisites: false,
            },
        ))
        .unwrap();
        assert_eq!(
            code, 1,
            "validate failures (invalid/error) unify on exit code 1; JSON status distinguishes"
        );
    }

    #[test]
    fn test_generate_exit_0() {
        let dir = tempdir();
        let code = run_with_args(cli(
            true,
            Commands::Generate {
                output: Some(dir.path().to_path_buf()),
                config_type: super::super::ConfigType::Minimal,
                env_example: false,
                schema: false,
            },
        ))
        .unwrap();
        assert_eq!(code, 0);
        // 模板确实落盘
        assert!(fs::read_dir(dir.path()).unwrap().count() > 0);
    }

    #[test]
    fn test_generate_schema_exit_0_writes_valid_json() {
        let dir = tempdir();
        let out = dir.path().join("schema.json");
        let code = run_with_args(cli(
            true,
            Commands::Generate {
                output: Some(out.clone()),
                config_type: super::super::ConfigType::Minimal,
                env_example: false,
                schema: true,
            },
        ))
        .unwrap();
        assert_eq!(code, 0);
        // --schema 模式下 --output 即文件路径，且内容为合法 JSON Schema
        let parsed: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&out).expect("schema file must exist"))
                .expect("schema output must be valid JSON");
        assert!(
            parsed.get("properties").is_some(),
            "schema must expose config properties"
        );
    }

    #[test]
    fn test_decrypt_missing_input_is_error() {
        let code = run_with_args(cli(
            true,
            Commands::Decrypt {
                input: PathBuf::from("/nonexistent/inklog/file.enc"),
                output: None,
                key_env: "INKLOG_DECRYPT_KEY".to_string(),
                recursive: false,
                batch: false,
            },
        ));
        assert!(
            code.is_err(),
            "missing input is an error (run_cli maps it to exit 1)"
        );
    }

    #[test]
    fn test_query_no_matches_exit_2_via_run_with_args() {
        let dir = tempdir();
        let log = dir.path().join("app.log");
        fs::write(&log, "2026-09-11T01:00:00.000Z [INFO] a - ok\n").unwrap();
        let code = run_with_args(cli(
            true,
            Commands::Query {
                path: vec![log],
                since: None,
                until: None,
                level: None,
                grep: Some("no-such-keyword".to_string()),
                limit: 100,
                key_env: None,
            },
        ))
        .unwrap();
        assert_eq!(code, 2, "no matches must exit 2");
    }
}

#[cfg(all(test, feature = "cli"))]
mod verify_chain_tests {
    use super::*;

    /// ok 路径：真实轮转产物 manifest → 退出码 0；篡改后 → 退出码 2。
    #[test]
    fn test_verify_chain_ok_and_tampered_paths() {
        use inklog::support::audit_chain::ArchiveChain;

        let dir = tempfile::TempDir::new().unwrap();
        let manifest = dir.path().join("chained.chain.jsonl");

        // 用与 FileSink 审计链一致的语义构造真实链
        let key = "verify-chain-ok-test-key";
        let mut chain = ArchiveChain::new(key.as_bytes());
        let entry_json = |p: &str| {
            serde_json::json!({
                "path": p,
                "sha256": format!("{:064x}", p.len()),
                "timestamp": "2026-09-23T00:00:00+00:00",
            })
            .to_string()
        };
        let mut body = String::new();
        for p in ["chained_20260923_000001.log", "chained_20260923_000002.log"] {
            chain.append(&entry_json(p));
        }
        for e in chain.entries() {
            body.push_str(&serde_json::to_string(e).unwrap());
            body.push('\n');
        }
        std::fs::write(&manifest, &body).unwrap();

        unsafe {
            std::env::set_var("INKLOG_TEST_VERIFY_KEY", key);
        }
        let code = verify_chain_manifest(&manifest, "INKLOG_TEST_VERIFY_KEY").unwrap();
        assert_eq!(code, 0, "intact manifest must verify with exit code 0");

        // 篡改：改 sha256 键名破坏 HMAC
        let tampered = body.replace("sha256", "sha256x");
        let tampered_path = dir.path().join("tampered.chain.jsonl");
        std::fs::write(&tampered_path, tampered).unwrap();
        let code = verify_chain_manifest(&tampered_path, "INKLOG_TEST_VERIFY_KEY").unwrap();
        assert_eq!(code, 2, "tampered manifest must yield exit code 2");
        unsafe {
            std::env::remove_var("INKLOG_TEST_VERIFY_KEY");
        }
    }
}
