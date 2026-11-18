// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
use anyhow::{Context as _, Result};
use clap::Parser;
use inklog::support::audit_chain::{ArchiveChain, ArchiveChainEntry, ArchiveChainSeal};
use std::path::PathBuf;

use super::query::QueryArgs;
use super::{Cli, Commands, decrypt, doctor, generate, query, validate};

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

        Commands::VerifyChain {
            manifest,
            key_env,
            anchor,
            anchor_key_env,
            #[cfg(feature = "audit-anchor")]
            anchor_pubkey,
        } => {
            #[cfg(feature = "audit-anchor")]
            if let Some(pubkey) = anchor_pubkey.as_deref() {
                let report = verify_chain_with_anchor_pubkey(
                    &manifest,
                    || {
                        std::env::var(&key_env)
                            .with_context(|| inklog::i18n::tr("cli-verify-chain-err-key-missing"))
                            .map(String::into_bytes)
                    },
                    Some(pubkey),
                )?;
                print_verdict(&manifest, &report);
                if args.json {
                    println!("{}", report.json(&manifest));
                }
                return Ok(report.exit_code());
            }
            verify_chain_manifest(
                &manifest,
                &key_env,
                anchor.as_deref(),
                anchor_key_env.as_deref(),
                args.json,
            )
        }

        Commands::Doctor { security, config } => {
            if !security {
                return Err(anyhow::anyhow!(
                    "{}",
                    inklog::i18n::tr("cli-doctor-err-no-target")
                ));
            }
            doctor::run_doctor_security(config, args.json)
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

// 全子命令 --json + 稳定退出码（0/1/2）契约单测

/// `verify-chain` 校验结论。
///
/// 退出码只有 0（链完整）与 2（其余）两档，但运维必须区分「被人为篡改」
/// 与「封条缺失 / manifest 为空 / manifest 与封条不同代 / 校验期间发生
/// 轮转」——判定进 `ChainVerdict`，输出文案与单测断言共用同一份结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChainVerdict {
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
    pub(crate) fn exit_code(&self) -> i32 {
        match self {
            ChainVerdict::Intact => 0,
            _ => 2,
        }
    }

    /// `--json` `status` 字段值（链+封条段；机器可判的分段结论）。
    fn json_status(&self) -> &'static str {
        match self {
            ChainVerdict::Intact => "ok",
            ChainVerdict::Tampered => "tampered",
            ChainVerdict::Empty => "empty",
            ChainVerdict::SealUnreadable { .. } => "seal-unreadable",
            ChainVerdict::GenerationMismatch { .. } => "generation-mismatch",
        }
    }
}

/// 校验结论 + 判定依据。
///
/// `pub(crate)`：`verify_chain`/`verify_chain_pair` 下沉为可复用单元，
/// MCP 工具（change mcp-log-tools 的 inklog_audit_verify）直接消费。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ChainVerification {
    pub(crate) verdict: ChainVerdict,
    /// 参与判定的 manifest 条目数
    pub(crate) entry_count: usize,
    /// 首次读取的 manifest 与封条目目数不符、重读后配对一致——校验期间发生轮转
    pub(crate) rotated: bool,
    /// 与封条判定同一代的 entries 快照（锚校验段共用，避免二次读取错代）
    pub(crate) entries: Vec<ArchiveChainEntry>,
}

/// 一代 manifest + 封条的读取结果。
///
/// manifest 与封条是两次独立读取，而写入侧两文件分两次落盘（非原子成对
/// 发布），轮转可能落在两次读之间——快照承载「某次读取看到的一代」。
#[derive(Debug, Clone)]
pub(crate) struct ChainSnapshot {
    /// manifest 条目
    pub(crate) entries: Vec<ArchiveChainEntry>,
    /// 封条；`Err` 为读取或解析失败原因
    pub(crate) seal: Result<ArchiveChainSeal, String>,
}

/// `verify-chain`：重算 HMAC 链并核对 manifest 完整性，附加锚校验段。
///
/// 退出码契约：`0` = 链完整（含锚段通过/未锚定），`1` = 错误，
/// `2` = 篡改/校验失败（含锚段失败）。
fn verify_chain_manifest(
    manifest: &std::path::Path,
    key_env: &str,
    anchor: Option<&std::path::Path>,
    anchor_key_env: Option<&str>,
    json: bool,
) -> Result<i32> {
    // 锚密钥解析：--anchor-key-env 值为「持有锚密钥的环境变量名」（与
    // --key-env 同约定）；变量缺失 → None（锚文件存在时按无法验证处理）
    let anchor_key = anchor_key_env
        .and_then(|name| std::env::var(name).ok())
        .map(String::into_bytes);
    let report = verify_chain_with_anchors(
        manifest,
        || {
            std::env::var(key_env)
                .with_context(|| inklog::i18n::tr("cli-verify-chain-err-key-missing"))
                .map(String::into_bytes)
        },
        anchor,
        anchor_key,
    )?;
    print_verdict(manifest, &report);
    if json {
        println!("{}", report.json(manifest));
    }
    Ok(report.exit_code())
}

/// 锚校验段结论（verify-chain 第三段；`--json` 与 MCP 工具共用）。
///
/// fail-closed 原则与封条一致：锚文件存在但无法验证（密钥缺失/文件不可读）
/// 即无法证明完整性，按失败处理。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AnchorSection {
    /// 锚校验已执行（锚密钥可用）
    Verified(inklog::support::audit_chain::AnchorVerdict),
    /// 未锚定：默认探测无锚文件且未显式指定 --anchor（不影响既有退出码语义）
    NoAnchor,
    /// 锚文件存在但锚密钥不可用——无法验证
    NoKey,
    /// 锚文件不可读或含不可解析记录
    Unreadable { err: String },
}

impl AnchorSection {
    /// 锚段对整体退出码的覆盖：失败 → `Some(2)`；通过/未锚定 → `None`
    /// （沿用链判定退出码）。
    fn exit_code_override(&self) -> Option<i32> {
        match self {
            AnchorSection::Verified(v) if v.all_valid() => None,
            AnchorSection::NoAnchor => None,
            AnchorSection::Verified(_)
            | AnchorSection::NoKey
            | AnchorSection::Unreadable { .. } => Some(2),
        }
    }

    /// `--json` 字段：`(anchor_status, anchors_valid)`。`anchors_valid` 为
    /// `"N/M"` 计数字符串，仅在校验实际执行（`Verified`）时携带。
    fn json_fields(&self) -> (&'static str, Option<String>) {
        match self {
            AnchorSection::Verified(v) => {
                let counts = format!("{}/{}", v.valid, v.total);
                if v.all_valid() {
                    ("ok", Some(counts))
                } else {
                    ("fail", Some(counts))
                }
            }
            AnchorSection::NoAnchor => ("no-anchor", None),
            AnchorSection::NoKey | AnchorSection::Unreadable { .. } => ("fail", None),
        }
    }

    /// 人类可读的失败明细（fail 文案 `{ $detail }` 参数；数据性描述，
    /// 与 seal-unreadable 的 `{ $err }` 同为先例）。
    fn fail_detail(&self) -> String {
        match self {
            AnchorSection::Verified(v) => {
                let mut detail = format!("{}/{} anchors valid", v.valid, v.total);
                if let Some((position, index)) = v.first_mismatch {
                    detail.push_str(&format!(
                        "; first mismatch: anchor #{} (chain index {})",
                        position, index
                    ));
                }
                detail
            }
            AnchorSection::NoKey => "anchor key unavailable (--anchor-key-env)".to_string(),
            AnchorSection::Unreadable { err } => format!("anchor file unreadable: {err}"),
            AnchorSection::NoAnchor => String::new(),
        }
    }
}

/// verify-chain 分段报告：链+封条结论 + 锚校验段。
///
/// CLI（人类可读 / `--json`）与 MCP 工具（inklog_audit_verify 分段结论）
/// 共用同一判定，避免双实现漂移。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ChainReport {
    /// 链 + 封条段
    pub(crate) outcome: ChainVerification,
    /// 锚段
    pub(crate) anchor: AnchorSection,
}

impl ChainReport {
    /// 整体退出码：锚段失败覆盖为 2，否则沿用链判定退出码。
    pub(crate) fn exit_code(&self) -> i32 {
        self.outcome
            .verdict
            .exit_code()
            .max(self.anchor.exit_code_override().unwrap_or(0))
    }

    /// `--json` 分段结论（各段独立字段，机器可判；与 MCP 工具同构）。
    pub(crate) fn json(&self, manifest: &std::path::Path) -> serde_json::Value {
        let (anchor_status, anchors_valid) = self.anchor.json_fields();
        let mut value = serde_json::json!({
            "command": "verify-chain",
            "manifest": manifest.display().to_string(),
            "entries": self.outcome.entry_count,
            "status": self.outcome.verdict.json_status(),
            "anchor_status": anchor_status,
        });
        if let Some(valid) = anchors_valid {
            value["anchors_valid"] = serde_json::json!(valid);
        }
        value
    }
}

/// 校验主体（链 + 封条 + 锚段），CLI 与 MCP 工具共用的可复用单元。
///
/// `anchor_path` `None` = 默认探测 manifest 同目录 `<stem>.anchor.jsonl`；
/// `anchor_key` `None` = 锚密钥不可用（锚文件存在时按无法验证处理）。
pub(crate) fn verify_chain_with_anchors(
    manifest: &std::path::Path,
    key: impl FnOnce() -> Result<Vec<u8>>,
    anchor_path: Option<&std::path::Path>,
    anchor_key: Option<Vec<u8>>,
) -> Result<ChainReport> {
    let outcome = verify_chain(manifest, key)?;
    let anchor = verify_anchor_section(manifest, anchor_path, anchor_key, &outcome.entries);
    Ok(ChainReport { outcome, anchor })
}

/// 非对称锚校验（feature `audit-anchor`）：验侧只需 Ed25519 公钥 hex。
/// pubkey 无效（解析失败/格式错）→ 锚段按 fail-closed 处理。
#[cfg(feature = "audit-anchor")]
pub(crate) fn verify_chain_with_anchor_pubkey(
    manifest: &std::path::Path,
    key: impl FnOnce() -> Result<Vec<u8>>,
    pubkey_hex: Option<&str>,
) -> Result<ChainReport> {
    let outcome = verify_chain(manifest, key)?;
    let anchor = verify_anchor_section_pubkey(manifest, pubkey_hex, &outcome.entries);
    Ok(ChainReport { outcome, anchor })
}

/// 通用锚段执行：解析锚文件后用调用方提供的签名器逐锚校验。
fn verify_anchor_records_with_signer(
    manifest: &std::path::Path,
    anchor_path: Option<&std::path::Path>,
    entries: &[ArchiveChainEntry],
    signer: &dyn inklog::support::audit_chain::AnchorSigner,
) -> AnchorSection {
    let resolved = match anchor_path {
        Some(path) => Some(path.to_path_buf()),
        None => {
            let probe = inklog::support::audit_chain::anchor_path_for_manifest(manifest);
            probe.exists().then_some(probe)
        }
    };
    let Some(path) = resolved else {
        return AnchorSection::NoAnchor;
    };
    let records = match read_anchor_records(&path) {
        Ok(records) => records,
        Err(err) => return AnchorSection::Unreadable { err },
    };
    AnchorSection::Verified(inklog::support::audit_chain::verify_anchors(
        &records, entries, signer,
    ))
}

/// 锚校验段执行：探测/显式锚文件 → 读取 → `verify_anchors`。
///
/// 锚校验复用链判定读到的同一份 entries 快照——锚与链必须对同一代
/// manifest 判定，二次读取可能在轮转竞态下产生假失配。
fn verify_anchor_section(
    manifest: &std::path::Path,
    anchor_path: Option<&std::path::Path>,
    anchor_key: Option<Vec<u8>>,
    entries: &[ArchiveChainEntry],
) -> AnchorSection {
    let Some(key) = anchor_key else {
        // 无密钥时仍需判定是否存在锚文件（存在即 fail-closed，避免静默通过）
        let resolved = match anchor_path {
            Some(path) => Some(path.to_path_buf()),
            None => {
                let probe = inklog::support::audit_chain::anchor_path_for_manifest(manifest);
                probe.exists().then_some(probe)
            }
        };
        return if resolved.is_some() {
            AnchorSection::NoKey
        } else {
            AnchorSection::NoAnchor
        };
    };
    verify_anchor_records_with_signer(
        manifest,
        anchor_path,
        entries,
        &inklog::support::audit_chain::HmacAnchorSigner::new(key),
    )
}

/// 非对称验侧锚段（feature `audit-anchor`）：公钥无效按 Unreadable fail-closed。
#[cfg(feature = "audit-anchor")]
fn verify_anchor_section_pubkey(
    manifest: &std::path::Path,
    pubkey_hex: Option<&str>,
    entries: &[ArchiveChainEntry],
) -> AnchorSection {
    let Some(raw) = pubkey_hex else {
        let probe = inklog::support::audit_chain::anchor_path_for_manifest(manifest);
        return if probe.exists() {
            AnchorSection::NoKey
        } else {
            AnchorSection::NoAnchor
        };
    };
    let signer = match inklog::support::audit_chain::Ed25519AnchorSigner::from_pubkey_hex(raw) {
        Ok(signer) => signer,
        Err(err) => {
            return AnchorSection::Unreadable {
                err: err.to_string(),
            };
        }
    };
    verify_anchor_records_with_signer(manifest, None, entries, &signer)
}

/// 读取并解析锚文件（JSONL，每行一条 [`inklog::support::audit_chain::AnchorRecord`]）。
fn read_anchor_records(
    path: &std::path::Path,
) -> std::result::Result<Vec<inklog::support::audit_chain::AnchorRecord>, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut records = Vec::new();
    for (lineno, line) in data.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let record: inklog::support::audit_chain::AnchorRecord =
            serde_json::from_str(line).map_err(|e| format!("line {}: {e}", lineno + 1))?;
        records.push(record);
    }
    Ok(records)
}

/// 校验主体：密钥惰性求值。
///
/// 空 manifest 的判据与密钥无关——先判空再取密钥：缺 key 且 manifest 为空
/// 时也稳定给退出码 2 与「空 manifest」诊断（诊断优先级：空链先于密钥缺失）。
pub(crate) fn verify_chain(
    manifest: &std::path::Path,
    key: impl FnOnce() -> Result<Vec<u8>>,
) -> Result<ChainVerification> {
    let snapshot = read_chain_snapshot(manifest)?;
    if snapshot.entries.is_empty() {
        return Ok(ChainVerification {
            verdict: ChainVerdict::Empty,
            entry_count: 0,
            rotated: false,
            entries: Vec::new(),
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
pub(crate) fn verify_chain_pair(
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
                entries,
            });
        }
    };
    if seal.count == entries.len() as u64 {
        let verdict = chain_verdict(&entries, &seal, key);
        return Ok(ChainVerification {
            verdict,
            entry_count: entries.len(),
            rotated: false,
            entries,
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
                entries,
            });
        }
    };
    if seal.count == entries.len() as u64 {
        // 重读后配对一致：校验期间发生轮转，按重读后的实际结果判定
        let verdict = chain_verdict(&entries, &seal, key);
        return Ok(ChainVerification {
            verdict,
            entry_count: entries.len(),
            rotated: true,
            entries,
        });
    }
    // 重读后仍不配对：manifest 与封条不属于同一代——无法证明完整性
    Ok(ChainVerification {
        verdict: ChainVerdict::GenerationMismatch {
            seal_count: seal.count,
        },
        entry_count: entries.len(),
        rotated: false,
        entries,
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

/// 输出校验结论（本地化文案）：链+封条段 + 锚校验段。
fn print_verdict(manifest: &std::path::Path, report: &ChainReport) {
    let outcome = &report.outcome;
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
    print_anchor_section(&report.anchor);
}

/// 输出锚校验段（本地化文案）。
fn print_anchor_section(section: &AnchorSection) {
    match section {
        AnchorSection::Verified(v) if v.all_valid() => {
            let mut args = inklog::i18n::MsgArgs::new();
            args.set("count", format!("{}/{}", v.valid, v.total));
            println!(
                "{}",
                inklog::i18n::tr_args("cli-verify-chain-anchor-ok", args)
            );
        }
        AnchorSection::Verified(_) | AnchorSection::NoKey | AnchorSection::Unreadable { .. } => {
            let mut args = inklog::i18n::MsgArgs::new();
            args.set("detail", section.fail_detail());
            println!(
                "{}",
                inklog::i18n::tr_args("cli-verify-chain-anchor-fail", args)
            );
        }
        AnchorSection::NoAnchor => {
            println!("{}", inklog::i18n::tr("cli-verify-chain-anchor-no-anchor"));
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

/// 锚校验段（verify-chain 第三段）单测——T006。
///
/// 密钥与锚密钥一律显式传入：进程内不改环境变量（edition 2024 下
/// POSIX `setenv` 非线程安全，与其他线程 `getenv` 并发即未定义行为）。
#[cfg(test)]
mod anchor_verify_tests {
    use super::*;
    use inklog::support::audit_chain::{
        HmacAnchorSigner, anchor_path_for_manifest, create_anchor_record,
    };

    /// 构造 n 条条目的链并落盘 manifest + 封条，返回（manifest, entries）。
    fn build_chain(
        dir: &std::path::Path,
        stem: &str,
        key: &[u8],
        n: usize,
    ) -> (PathBuf, Vec<ArchiveChainEntry>) {
        let mut chain = ArchiveChain::new(key);
        for i in 0..n {
            chain.append(&format!("{{\"n\":{i}}}"));
        }
        let manifest = dir.join(format!("{stem}.chain.jsonl"));
        let body: String = chain
            .entries()
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&manifest, body).unwrap();
        let seal_path = inklog::support::audit_chain::seal_path_for_manifest(&manifest);
        std::fs::write(
            &seal_path,
            serde_json::to_string(&chain.seal().expect("seal")).unwrap(),
        )
        .unwrap();
        (manifest, chain.entries().to_vec())
    }

    /// 为链条目生成合法锚记录并落盘（JSONL，路径 = 默认探测位）。
    fn write_anchor_file(manifest: &std::path::Path, entries: &[ArchiveChainEntry], key: &[u8]) {
        let signer = HmacAnchorSigner::new(key.to_vec());
        let records: Vec<_> = entries
            .iter()
            .enumerate()
            .map(|(i, e)| create_anchor_record(&signer, i as u64, &e.hmac, i as u64 + 1).unwrap())
            .collect();
        let body = records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(anchor_path_for_manifest(manifest), body).unwrap();
    }

    /// 锚全通过 → 退出码不变（链 intact → 0），json 锚段 ok 且 3/3。
    #[test]
    fn test_anchor_all_valid_keeps_exit_code() {
        let dir = tempfile::TempDir::new().unwrap();
        let (manifest, entries) = build_chain(dir.path(), "anchored", b"chain-key-t006", 3);
        write_anchor_file(&manifest, &entries, b"anchor-key-t006");

        let report = verify_chain_with_anchors(
            &manifest,
            || Ok(b"chain-key-t006".to_vec()),
            None,
            Some(b"anchor-key-t006".to_vec()),
        )
        .unwrap();
        let AnchorSection::Verified(verdict) = &report.anchor else {
            panic!("anchor file present + key available: anchor section must verify");
        };
        assert!(verdict.all_valid(), "all anchors must pass");
        assert_eq!(report.exit_code(), 0, "valid anchors must keep exit code 0");

        let json = report.json(&manifest);
        assert_eq!(json["anchor_status"], "ok");
        assert_eq!(json["anchors_valid"], "3/3");
    }

    /// 持链钥攻击者重写全部条目并重签 seal（manifest/seal 全部合法），
    /// 沿用旧锚 → 锚段失配：退出码非 0 且 json anchors_valid=="0/3"。
    #[test]
    fn test_anchor_mismatch_fails_with_anchors_valid_json() {
        let dir = tempfile::TempDir::new().unwrap();

        // 旧链与旧锚（锚记录引用旧链头）
        let mut old = ArchiveChain::new(b"chain-key-t006");
        for i in 0..3 {
            old.append(&format!("{{\"n\":{i}}}"));
        }
        let signer = HmacAnchorSigner::new(b"anchor-key-t006".to_vec());
        let stale_records: Vec<_> = old
            .entries()
            .iter()
            .enumerate()
            .map(|(i, e)| create_anchor_record(&signer, i as u64, &e.hmac, i as u64 + 1).unwrap())
            .collect();

        // 攻击者用同一链钥重写条目内容，重签 manifest + seal（链内校验全通过）
        let mut rewritten = ArchiveChain::new(b"chain-key-t006");
        for i in 0..3 {
            rewritten.append(&format!("{{\"n\":{i},\"rewritten\":true}}"));
        }
        let manifest = dir.path().join("rewritten.chain.jsonl");
        let body: String = rewritten
            .entries()
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&manifest, body).unwrap();
        let seal_path = inklog::support::audit_chain::seal_path_for_manifest(&manifest);
        std::fs::write(
            &seal_path,
            serde_json::to_string(&rewritten.seal().expect("seal")).unwrap(),
        )
        .unwrap();
        let anchor_body = stale_records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(anchor_path_for_manifest(&manifest), anchor_body).unwrap();

        let report = verify_chain_with_anchors(
            &manifest,
            || Ok(b"chain-key-t006".to_vec()),
            None,
            Some(b"anchor-key-t006".to_vec()),
        )
        .unwrap();
        assert_eq!(
            report.exit_code(),
            2,
            "rewritten chain with stale anchors must fail closed"
        );

        let json = report.json(&manifest);
        assert_eq!(json["anchor_status"], "fail");
        assert_eq!(json["anchors_valid"], "0/3");
        assert_eq!(
            json["status"], "ok",
            "chain+seal section alone stays intact: only the anchor section fails"
        );
    }

    /// 无锚文件 → 锚段报「未锚定」，不改变既有退出码语义；json 无 anchors_valid。
    #[test]
    fn test_no_anchor_file_keeps_exit_code() {
        let dir = tempfile::TempDir::new().unwrap();
        let (manifest, _) = build_chain(dir.path(), "noanchor", b"chain-key-t006", 2);

        let report =
            verify_chain_with_anchors(&manifest, || Ok(b"chain-key-t006".to_vec()), None, None)
                .unwrap();
        assert_eq!(report.anchor, AnchorSection::NoAnchor);
        assert_eq!(
            report.exit_code(),
            0,
            "not-anchored must not alter exit code"
        );

        let json = report.json(&manifest);
        assert_eq!(json["anchor_status"], "no-anchor");
        assert!(
            json.get("anchors_valid").is_none(),
            "anchors_valid must be absent when no anchor file exists"
        );
    }

    /// 显式 --anchor 指向缺失文件：无法验证用户明确要求的锚 → fail-closed。
    #[test]
    fn test_explicit_anchor_missing_fails_closed() {
        let dir = tempfile::TempDir::new().unwrap();
        let (manifest, _) = build_chain(dir.path(), "explicit", b"chain-key-t006", 1);
        let missing = dir.path().join("absent.anchor.jsonl");

        let report = verify_chain_with_anchors(
            &manifest,
            || Ok(b"chain-key-t006".to_vec()),
            Some(&missing),
            Some(b"anchor-key-t006".to_vec()),
        )
        .unwrap();
        assert!(
            matches!(report.anchor, AnchorSection::Unreadable { .. }),
            "explicit anchor path that cannot be read must be unreadable"
        );
        assert_eq!(report.exit_code(), 2);
    }

    /// 锚文件存在但锚密钥不可用 → 无法验证（fail-closed，退出码 2）。
    #[test]
    fn test_anchor_without_key_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let (manifest, entries) = build_chain(dir.path(), "nokey", b"chain-key-t006", 2);
        write_anchor_file(&manifest, &entries, b"anchor-key-t006");

        let report =
            verify_chain_with_anchors(&manifest, || Ok(b"chain-key-t006".to_vec()), None, None)
                .unwrap();
        assert_eq!(report.anchor, AnchorSection::NoKey);
        assert_eq!(
            report.exit_code(),
            2,
            "present anchors without a key cannot be verified: fail closed"
        );
    }
}

/// T010: verify-chain `--anchor-pubkey` 的非对称验签路径（feature 门控）。
#[cfg(all(test, feature = "audit-anchor"))]
mod ed25519_anchor_verify_tests {
    use super::*;
    use inklog::support::audit_chain::{
        Ed25519AnchorSigner, anchor_path_for_manifest, create_anchor_record,
    };

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn build_chain(
        dir: &std::path::Path,
        key: &[u8],
        n: usize,
    ) -> (PathBuf, Vec<ArchiveChainEntry>) {
        let mut chain = ArchiveChain::new(key);
        for i in 0..n {
            chain.append(&format!("{{\"n\":{i}}}"));
        }
        let manifest = dir.join("ed25519.chain.jsonl");
        let body = chain
            .entries()
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&manifest, body).unwrap();
        std::fs::write(
            inklog::support::audit_chain::seal_path_for_manifest(&manifest),
            serde_json::to_string(&chain.seal().expect("seal")).unwrap(),
        )
        .unwrap();
        (manifest, chain.entries().to_vec())
    }

    fn write_ed25519_anchors(
        manifest: &std::path::Path,
        entries: &[ArchiveChainEntry],
        signer: &Ed25519AnchorSigner,
    ) {
        let records: Vec<_> = entries
            .iter()
            .enumerate()
            .map(|(i, e)| create_anchor_record(signer, i as u64, &e.hmac, i as u64 + 1).unwrap())
            .collect();
        let body = records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(anchor_path_for_manifest(manifest), body).unwrap();
    }

    #[test]
    fn test_ed25519_pubkey_verifies_anchors() {
        let dir = tempfile::tempdir().unwrap();
        let (manifest, entries) = build_chain(dir.path(), b"chain-key-t010", 3);
        let signer = Ed25519AnchorSigner::from_seed([0x42; 32]).unwrap();
        write_ed25519_anchors(&manifest, &entries, &signer);

        let pubkey_hex = hex(&signer.verifying_key_bytes());
        let report = verify_chain_with_anchor_pubkey(
            &manifest,
            || Ok(b"chain-key-t010".to_vec()),
            Some(&pubkey_hex),
        )
        .unwrap();
        let AnchorSection::Verified(verdict) = &report.anchor else {
            panic!("valid pubkey must trigger verification");
        };
        assert!(verdict.all_valid());
        assert_eq!(report.json(&manifest)["anchors_valid"], "3/3");
        assert_eq!(report.exit_code(), 0);
    }

    #[test]
    fn test_ed25519_wrong_pubkey_fails() {
        let dir = tempfile::tempdir().unwrap();
        let (manifest, entries) = build_chain(dir.path(), b"chain-key-t010", 2);
        let signer = Ed25519AnchorSigner::from_seed([0x42; 32]).unwrap();
        write_ed25519_anchors(&manifest, &entries, &signer);

        let wrong = Ed25519AnchorSigner::from_seed([0x43; 32]).unwrap();
        let wrong_hex = hex(&wrong.verifying_key_bytes());
        let report = verify_chain_with_anchor_pubkey(
            &manifest,
            || Ok(b"chain-key-t010".to_vec()),
            Some(&wrong_hex),
        )
        .unwrap();
        assert_ne!(report.exit_code(), 0, "wrong pubkey must fail verification");
    }
}
