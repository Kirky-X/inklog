// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
use anyhow::{Context as _, Result};
use clap::Parser;
use inklog::support::audit_chain::{ArchiveChain, ArchiveChainEntry, ArchiveChainSeal};
use std::path::PathBuf;

use super::query::QueryArgs;
use super::{Cli, Commands, decrypt, generate, query, validate};

/// 展开 anyhow 因果链为单行错误信息：每层 `InklogError` 走本地化文案，
/// 其余层按 Display 原样。避免 `to_string()` 只取最外层而吞掉底层明细
/// （如密钥长度、密码长度诊断）。
pub(super) fn format_error_chain(e: &anyhow::Error) -> String {
    e.chain()
        .map(|cause| {
            cause
                .downcast_ref::<inklog::InklogError>()
                .map(|err| err.localized_message())
                .unwrap_or_else(|| cause.to_string())
        })
        .collect::<Vec<_>>()
        .join(": ")
}

pub fn run_cli() -> i32 {
    let args = Cli::parse();
    let json = args.json;
    match run_with_args(args) {
        Ok(code) => code,
        Err(e) => {
            let msg = format_error_chain(&e);
            if json {
                eprintln!(
                    "{}",
                    serde_json::json!({ "status": "error", "message": msg })
                );
            } else {
                // 本地化输出：InklogError 走 FTL 前缀（localized_message 路径）；
                // 其余错误（CLI 层多为已本地化的 anyhow 消息）按 Display 原样输出
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

// 全子命令 --json + 稳定退出码（0/1/2）契约单测

/// `verify-chain` 校验结论。
///
/// 退出码只有 0（链完整）与 2（其余）两档，但运维必须区分「被人为篡改」
/// 与「封条缺失 / manifest 为空 / manifest 与封条不同代 / 校验期间发生
/// 轮转」——判定进 `ChainVerdict`，输出文案与单测断言共用同一份结论。
#[derive(Debug, Clone, PartialEq, Eq)]
enum ChainVerdict {
    /// 链完整
    Intact,
    /// 链被篡改（删除/重排/内容修改）
    Tampered,
    /// manifest 为空（0 条目）——无法证明完整性
    Empty,
    /// 封条缺失或不可解析——无法判定完整性
    SealUnreadable { err: String },
    /// manifest 与封条覆盖的条目数不一致（重读后仍不一致）
    GenerationMismatch { seal_count: u64 },
}

impl ChainVerdict {
    /// 退出码：`0` = 链完整，`2` = 其余（无法证明完整性）。
    fn exit_code(&self) -> i32 {
        match self {
            ChainVerdict::Intact => 0,
            _ => 2,
        }
    }
}

/// 校验结论 + 判定依据。
#[derive(Debug, Clone, PartialEq, Eq)]
struct ChainVerification {
    verdict: ChainVerdict,
    /// 参与判定的 manifest 条目数
    entry_count: usize,
    /// 首次读取的 manifest 与封条目目数不符、重读后配对一致——校验期间发生轮转
    rotated: bool,
}

/// 一代 manifest + 封条的读取结果。
///
/// manifest 与封条是两次独立读取，而写入侧两文件分两次落盘（非原子成对
/// 发布），轮转可能落在两次读之间——快照承载「某次读取看到的一代」。
#[derive(Debug, Clone)]
struct ChainSnapshot {
    /// manifest 条目
    entries: Vec<ArchiveChainEntry>,
    /// 封条；`Err` 为读取或解析失败原因
    seal: Result<ArchiveChainSeal, String>,
}

/// `verify-chain`：重算 HMAC 链并核对 manifest 完整性。
///
/// 退出码契约：`0` = 链完整，`1` = 错误，`2` = 篡改/校验失败。
fn verify_chain_manifest(manifest: &std::path::Path, key_env: &str) -> Result<i32> {
    let outcome = verify_chain(manifest, || {
        std::env::var(key_env)
            .with_context(|| inklog::i18n::tr("cli-verify-chain-err-key-missing"))
            .map(String::into_bytes)
    })?;
    print_verdict(manifest, &outcome);
    Ok(outcome.verdict.exit_code())
}

/// 校验主体：密钥惰性求值。
///
/// 空 manifest 的判据与密钥无关——先判空再取密钥：缺 key 且 manifest 为空
/// 时也稳定给退出码 2 与「空 manifest」诊断（诊断优先级：空链先于密钥缺失）。
fn verify_chain(
    manifest: &std::path::Path,
    key: impl FnOnce() -> Result<Vec<u8>>,
) -> Result<ChainVerification> {
    let snapshot = read_chain_snapshot(manifest)?;
    if snapshot.entries.is_empty() {
        return Ok(ChainVerification {
            verdict: ChainVerdict::Empty,
            entry_count: 0,
            rotated: false,
        });
    }
    let key = key()?;
    verify_chain_pair(&key, snapshot, || read_chain_snapshot(manifest))
}

/// 密钥显式传入的校验入口：不经环境变量取密钥——测试直接传字面量密钥，
/// 进程内不再改环境变量（edition 2024 下 POSIX `setenv` 非线程安全，与其他
/// 线程的 `getenv` 并发即未定义行为）。
#[cfg(test)]
fn verify_chain_with_key(manifest: &std::path::Path, key: &[u8]) -> Result<ChainVerification> {
    verify_chain(manifest, || Ok(key.to_vec()))
}

/// 判定 manifest 与封条是否一致（`entries` 非空，空判在 [`verify_chain`] 完成）。
///
/// 条目数不符时用 `reread` 重读一次再判定：写入侧 manifest 与封条分两次
/// 落盘，轮转落在两次读之间即读到「manifest=N 而 seal=N+1」这类错配，
/// 不重读就判篡改会把写入侧的不一致误报为人为篡改。
fn verify_chain_pair(
    key: &[u8],
    first: ChainSnapshot,
    reread: impl FnOnce() -> Result<ChainSnapshot>,
) -> Result<ChainVerification> {
    let ChainSnapshot { entries, seal } = first;
    let seal = match seal {
        Ok(seal) => seal,
        Err(err) => {
            return Ok(ChainVerification {
                verdict: ChainVerdict::SealUnreadable { err },
                entry_count: entries.len(),
                rotated: false,
            });
        }
    };
    if seal.count == entries.len() as u64 {
        return Ok(ChainVerification {
            verdict: chain_verdict(&entries, &seal, key),
            entry_count: entries.len(),
            rotated: false,
        });
    }

    // 条目数不符：重读一次（轮转可能落在两次读之间）
    let ChainSnapshot { entries, seal } = reread()?;
    let seal = match seal {
        Ok(seal) => seal,
        Err(err) => {
            return Ok(ChainVerification {
                verdict: ChainVerdict::SealUnreadable { err },
                entry_count: entries.len(),
                rotated: false,
            });
        }
    };
    if seal.count == entries.len() as u64 {
        // 重读后配对一致：校验期间发生轮转，按重读后的实际结果判定
        return Ok(ChainVerification {
            verdict: chain_verdict(&entries, &seal, key),
            entry_count: entries.len(),
            rotated: true,
        });
    }
    // 重读后仍不配对：manifest 与封条不属于同一代——无法证明完整性
    Ok(ChainVerification {
        verdict: ChainVerdict::GenerationMismatch {
            seal_count: seal.count,
        },
        entry_count: entries.len(),
        rotated: false,
    })
}

/// 链内自洽 + 封条比对 → 判定。
fn chain_verdict(
    entries: &[ArchiveChainEntry],
    seal: &ArchiveChainSeal,
    key: &[u8],
) -> ChainVerdict {
    let intact =
        ArchiveChain::verify_entries(entries, key) && ArchiveChain::verify_seal(seal, entries, key);
    if intact {
        ChainVerdict::Intact
    } else {
        ChainVerdict::Tampered
    }
}

/// 读取当前磁盘上的一代 manifest + 封条。
fn read_chain_snapshot(manifest: &std::path::Path) -> Result<ChainSnapshot> {
    let entries = read_manifest_entries(manifest)?;
    let seal_path = inklog::support::audit_chain::seal_path_for_manifest(manifest);
    let seal = read_seal(&seal_path);
    Ok(ChainSnapshot { entries, seal })
}

/// 读取并解析 manifest（JSONL，每行一个链条目）。
fn read_manifest_entries(manifest: &std::path::Path) -> Result<Vec<ArchiveChainEntry>> {
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
    Ok(entries)
}

/// 读取封条（链外锚点）：条目总数 + 尾哈希。manifest 被截尾到非空前缀时
/// 链内校验仍自洽，只有与封条比对才能判定截断。封条缺失/不可解析即
/// 无法判定完整性——删掉封条是绕过截尾检测的最短路径，按失败处理。
///
/// 威胁模型边界：封条与 manifest 同目录同可写域——把某一代的 manifest 与
/// 当时合法的封条一并回滚对本校验不可见。封条只挡单侧截尾，不挡成对回滚。
fn read_seal(seal_path: &std::path::Path) -> Result<ArchiveChainSeal, String> {
    std::fs::read_to_string(seal_path)
        .map_err(|e| e.to_string())
        .and_then(|raw| serde_json::from_str::<ArchiveChainSeal>(&raw).map_err(|e| e.to_string()))
}

/// 输出校验结论（本地化文案）。
fn print_verdict(manifest: &std::path::Path, outcome: &ChainVerification) {
    if outcome.rotated {
        println!(
            "{}: {}",
            inklog::i18n::tr("cli-verify-chain-rotated-during-verify"),
            manifest.display()
        );
    }
    match &outcome.verdict {
        ChainVerdict::Intact => {
            let mut args = inklog::i18n::MsgArgs::new();
            args.set("chain", inklog::i18n::tr("cli-verify-chain-ok"));
            args.set("path", manifest.display().to_string());
            args.set("count", outcome.entry_count);
            println!(
                "{}",
                inklog::i18n::tr_args("cli-verify-chain-ok-detail", args)
            );
        }
        ChainVerdict::Tampered => {
            println!(
                "{}: {}",
                inklog::i18n::tr("cli-verify-chain-tampered"),
                manifest.display()
            );
        }
        ChainVerdict::Empty => {
            println!(
                "{}: {}",
                inklog::i18n::tr("cli-verify-chain-empty"),
                manifest.display()
            );
        }
        ChainVerdict::SealUnreadable { err } => {
            let seal_path = inklog::support::audit_chain::seal_path_for_manifest(manifest);
            let mut args = inklog::i18n::MsgArgs::new();
            args.set("path", seal_path.display().to_string());
            args.set("err", err);
            println!(
                "{}: {}",
                inklog::i18n::tr_args("cli-verify-chain-seal-unreadable", args),
                manifest.display()
            );
        }
        ChainVerdict::GenerationMismatch { seal_count } => {
            let mut args = inklog::i18n::MsgArgs::new();
            args.set("seal_count", seal_count);
            args.set("count", outcome.entry_count);
            println!(
                "{}: {}",
                inklog::i18n::tr_args("cli-verify-chain-generation-mismatch", args),
                manifest.display()
            );
        }
    }
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
    use inklog::support::audit_chain::ArchiveChain;

    /// 落盘 manifest（JSONL，每行一个条目）。
    fn write_manifest(entries: &[ArchiveChainEntry], path: &std::path::Path) {
        let body: String = entries
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(path, body).unwrap();
    }

    /// 落盘封条（链外锚点）：与 manifest 成对存放。
    fn write_seal(seal: &ArchiveChainSeal, manifest: &std::path::Path) {
        let seal_path = inklog::support::audit_chain::seal_path_for_manifest(manifest);
        std::fs::write(&seal_path, serde_json::to_string(seal).unwrap()).unwrap();
    }

    /// ok 路径：真实轮转产物 manifest → 退出码 0；篡改后 → 退出码 2。
    #[test]
    fn test_verify_chain_ok_and_tampered_paths() {
        let dir = tempfile::TempDir::new().unwrap();
        let manifest = dir.path().join("chained.chain.jsonl");
        let key = b"verify-chain-ok-test-key";

        // 用与 FileSink 审计链一致的语义构造真实链
        let mut chain = ArchiveChain::new(key);
        let entry_json = |p: &str| {
            serde_json::json!({
                "path": p,
                "sha256": format!("{:064x}", p.len()),
                "timestamp": "2026-09-23T00:00:00+00:00",
            })
            .to_string()
        };
        for p in ["chained_20260923_000001.log", "chained_20260923_000002.log"] {
            chain.append(&entry_json(p));
        }
        write_manifest(chain.entries(), &manifest);
        write_seal(&chain.seal().expect("seal"), &manifest);

        let outcome = verify_chain_with_key(&manifest, key).unwrap();
        assert_eq!(
            outcome.verdict,
            ChainVerdict::Intact,
            "intact manifest must verify"
        );
        assert_eq!(
            outcome.verdict.exit_code(),
            0,
            "intact manifest must exit 0"
        );

        // 篡改：改事件内容破坏 HMAC
        let tampered_path = dir.path().join("tampered.chain.jsonl");
        let mut tampered_entries = chain.entries().to_vec();
        tampered_entries[0].event = tampered_entries[0].event.replace("sha256", "sha256x");
        write_manifest(&tampered_entries, &tampered_path);
        write_seal(&chain.seal().expect("seal"), &tampered_path);

        let outcome = verify_chain_with_key(&tampered_path, key).unwrap();
        assert_eq!(
            outcome.verdict,
            ChainVerdict::Tampered,
            "tampered manifest must be detected"
        );
        assert_eq!(
            outcome.verdict.exit_code(),
            2,
            "tampered manifest must exit 2"
        );
    }

    /// 截尾到非空前缀：链内自洽，链外锚点（封条）必须判失败。
    #[test]
    fn test_verify_chain_detects_truncation() {
        let dir = tempfile::TempDir::new().unwrap();
        let manifest = dir.path().join("truncated.chain.jsonl");
        let key = b"verify-chain-truncation-test-key";

        let mut chain = ArchiveChain::new(key);
        for i in 0..3 {
            chain.append(&format!("{{\"n\":{i}}}"));
        }
        let seal = chain.seal().expect("seal");
        // 只落盘前两条（截尾），封条仍是完整链的封条
        write_manifest(&chain.entries()[..2], &manifest);
        write_seal(&seal, &manifest);

        let outcome = verify_chain_with_key(&manifest, key).unwrap();
        // 封条覆盖 3 条、manifest 只有 2 条：重读后仍不配对 → 无法证明完整性
        assert_eq!(
            outcome.verdict,
            ChainVerdict::GenerationMismatch { seal_count: 3 },
            "truncated manifest must not be reported as an intact chain"
        );
        assert_eq!(
            outcome.verdict.exit_code(),
            2,
            "truncated manifest must yield exit code 2"
        );
    }

    /// 封条缺失：无法判定完整性——按失败处理（fail-closed）。
    #[test]
    fn test_verify_chain_fails_without_seal() {
        let dir = tempfile::TempDir::new().unwrap();
        let manifest = dir.path().join("noseal.chain.jsonl");
        let key = b"verify-chain-noseal-test-key";

        let mut chain = ArchiveChain::new(key);
        chain.append("{\"n\":0}");
        write_manifest(chain.entries(), &manifest);
        // 封条不存在

        let outcome = verify_chain_with_key(&manifest, key).unwrap();
        assert!(
            matches!(outcome.verdict, ChainVerdict::SealUnreadable { .. }),
            "missing seal must be reported as unreadable"
        );
        assert_eq!(
            outcome.verdict.exit_code(),
            2,
            "missing seal must yield exit code 2"
        );
    }

    /// 校验期间发生轮转：manifest 与封条是两次独立读取，写入侧两文件非原子
    /// 成对发布，轮转落在两次读之间即读到「manifest=N 而 seal=N+1」的错配
    /// ——必须重读再判定，不得误报篡改。
    #[test]
    fn test_verify_chain_rotation_during_verify_is_not_tampering() {
        let dir = tempfile::TempDir::new().unwrap();
        let manifest = dir.path().join("rotating.chain.jsonl");
        let key = b"verify-chain-rotation-test-key";

        let mut chain = ArchiveChain::new(key);
        for i in 0..3 {
            chain.append(&format!("{{\"n\":{i}}}"));
        }
        let seal = chain.seal().expect("seal");
        // 磁盘状态：manifest 还是轮转前（2 条），封条已是轮转后（覆盖 3 条）
        write_manifest(&chain.entries()[..2], &manifest);
        write_seal(&seal, &manifest);

        // 重读时轮转已完成：manifest 3 条 + 覆盖 3 条的封条 → 配对一致
        let after_rotation = ChainSnapshot {
            entries: chain.entries().to_vec(),
            seal: Ok(seal.clone()),
        };
        let first = read_chain_snapshot(&manifest).unwrap();
        // 只看首次快照就判定：错配会被判成篡改——这正是轮转期间的误报路径
        assert_eq!(
            chain_verdict(
                &first.entries,
                first.seal.as_ref().expect("seal readable"),
                key
            ),
            ChainVerdict::Tampered,
            "the mismatched pair alone must be judged as tampering"
        );
        // 重读后配对一致 → 判为校验期间发生轮转，按重读后的结果给结论
        let outcome = verify_chain_pair(key, first, || Ok(after_rotation)).unwrap();
        assert!(outcome.rotated, "the rotation must be detected");
        assert_eq!(
            outcome.verdict,
            ChainVerdict::Intact,
            "rotation during verify must not be reported as tampering"
        );
        assert_eq!(
            outcome.verdict.exit_code(),
            0,
            "rotation during verify must not fail the verification"
        );
    }

    /// 空 manifest（0 条目）：无法证明完整性——按失败处理（fail-closed）。
    #[test]
    fn test_verify_chain_empty_manifest_fails_closed() {
        let dir = tempfile::TempDir::new().unwrap();
        let manifest = dir.path().join("empty.chain.jsonl");
        std::fs::write(&manifest, "").unwrap();

        let outcome = verify_chain_with_key(&manifest, b"verify-chain-empty-test-key").unwrap();
        assert_eq!(
            outcome.verdict,
            ChainVerdict::Empty,
            "empty manifest must be reported as empty"
        );
        assert_eq!(
            outcome.verdict.exit_code(),
            2,
            "empty manifest must yield exit code 2"
        );
    }

    /// 封条存在但 JSON 非法：与「封条缺失」同属无法判定完整性——退出码 2。
    #[test]
    fn test_verify_chain_invalid_seal_json_fails_closed() {
        let dir = tempfile::TempDir::new().unwrap();
        let manifest = dir.path().join("badseal.chain.jsonl");
        let key = b"verify-chain-badseal-test-key";

        let mut chain = ArchiveChain::new(key);
        chain.append("{\"n\":0}");
        write_manifest(chain.entries(), &manifest);
        let seal_path = inklog::support::audit_chain::seal_path_for_manifest(&manifest);
        std::fs::write(&seal_path, "{ not json").unwrap();

        let outcome = verify_chain_with_key(&manifest, key).unwrap();
        assert!(
            matches!(outcome.verdict, ChainVerdict::SealUnreadable { .. }),
            "unparsable seal must be reported as unreadable"
        );
        assert_eq!(
            outcome.verdict.exit_code(),
            2,
            "unparsable seal must yield exit code 2"
        );
    }
}
