// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! `inklog doctor --security`：安全姿态评估的 CLI 承载。
//!
//! 库侧评估在 `inklog::support::security::assess_security_posture`；本模块
//! 只做三件事：加载配置 → 渲染报告 → 退出码映射（sound → 0，否则 1，
//! 对齐 cli_impl 中 validate 失败返回 Ok(1) 的约定）。

use anyhow::{Context as _, Result};
use std::path::PathBuf;

use inklog::support::security::{PostureReport, PostureStatus, assess_security_posture};

/// 配置文件默认路径（对齐 `validate` 子命令行为）。
const DEFAULT_CONFIG_PATH: &str = "inklog_config.toml";

/// 执行 `doctor --security`。
///
/// 退出码：`0` = 姿态健康（无 Critical），`1` = 存在 Critical 或发生错误
/// （错误经 `run_cli` 统一映射为 1）。
pub(crate) fn run_doctor_security(config_path: Option<PathBuf>, json: bool) -> Result<i32> {
    let config_path = config_path.unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH));
    let config = load_config(&config_path)?;
    let report = assess_security_posture(&config);

    if json {
        println!("{}", build_json_report(&report, &config_path));
    } else {
        print_human_report(&report);
    }
    Ok(i32::from(!report.is_sound()))
}

/// 读取并解析配置（复用 `InklogConfig::from_str` 的解析+校验路径）。
///
/// 文件缺失或配置非法 → Err（与 `validate` 的显性错误行为一致）。
fn load_config(config_path: &std::path::Path) -> Result<inklog::InklogConfig> {
    if !config_path.exists() {
        let mut args = inklog::i18n::MsgArgs::new();
        args.set("path", config_path.display().to_string());
        return Err(anyhow::anyhow!(
            "{}",
            inklog::i18n::tr_args("cli-err-config-not-exist", args)
        ));
    }
    let content = std::fs::read_to_string(config_path)
        .with_context(|| inklog::i18n::tr("cli-err-read-config"))?;
    content
        .parse()
        .with_context(|| inklog::i18n::tr("cli-err-parse-toml"))
}

/// `--json` 结构化输出（机器可读契约）。
fn build_json_report(report: &PostureReport, config_path: &std::path::Path) -> serde_json::Value {
    serde_json::json!({
        "command": "doctor",
        "target": "security",
        "config": config_path.display().to_string(),
        "checks": report.checks,
        "critical": report.critical_count(),
        "warn": report.warn_count(),
        "sound": report.is_sound(),
    })
}

/// 人类可读输出：逐行 `✓/!/✗/- capability: detail`，未达标项附修复提示。
fn print_human_report(report: &PostureReport) {
    for check in &report.checks {
        let status_key = match check.status {
            PostureStatus::Ok => "cli-doctor-check-ok",
            PostureStatus::Warn => "cli-doctor-check-warn",
            PostureStatus::Critical => "cli-doctor-check-critical",
            PostureStatus::Unavailable => "cli-doctor-check-unavailable",
        };
        let mut args = inklog::i18n::MsgArgs::new();
        args.set("capability", inklog::i18n::tr(check.capability));
        args.set(
            "detail",
            inklog::i18n::tr_args(check.detail_key, clone_args(&check.detail_args)),
        );
        println!("{}", inklog::i18n::tr_args(status_key, args));
        if check.status != PostureStatus::Ok
            && let Some(remedy) = check.remedy_key
        {
            println!("  → {}", inklog::i18n::tr(remedy));
        }
    }
    let mut args = inklog::i18n::MsgArgs::new();
    args.set("critical", report.critical_count());
    args.set("warn", report.warn_count());
    println!(
        "{}",
        inklog::i18n::tr_args(
            if report.is_sound() {
                "cli-doctor-summary-ok"
            } else {
                "cli-doctor-summary-unsound"
            },
            args,
        )
    );
}

/// `tr_args` 消费 MsgArgs；渲染用副本避免移动借用中的字段。
fn clone_args(args: &inklog::i18n::MsgArgs) -> inklog::i18n::MsgArgs {
    let mut out = inklog::i18n::MsgArgs::new();
    for (k, v) in args.to_map() {
        out.set(&k, v);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn write_config(dir: &tempfile::TempDir, name: &str, body: &str) -> PathBuf {
        let path = dir.path().join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    /// 合法最小配置：无 file sink → 无 Critical → 退出码 0（env 无关）。
    #[test]
    fn test_ok_scenario_exits_0() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_config(&dir, "ok.toml", "[global]\nlevel = \"info\"\n");
        let code = run_doctor_security(Some(cfg), false).unwrap();
        assert_eq!(code, 0, "no critical checks must exit 0");
    }

    /// 开审计链但无 INKLOG_AUDIT_KEY → Critical → 退出码 1。
    #[test]
    #[serial]
    fn test_critical_scenario_exits_1() {
        unsafe {
            std::env::remove_var("INKLOG_AUDIT_KEY");
        }
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_config(
            &dir,
            "critical.toml",
            "[file_sink]\nenabled = true\npath = \"logs/app.log\"\naudit_chain_enabled = true\n",
        );
        let code = run_doctor_security(Some(cfg), false).unwrap();
        assert_eq!(code, 1, "audit chain without key must exit 1");
    }

    /// JSON 输出可被 serde_json 解析且契约字段齐全。
    #[test]
    fn test_json_report_contract() {
        let config: inklog::InklogConfig = "[global]\nlevel = \"info\"\n".parse().unwrap();
        let report = assess_security_posture(&config);
        let json = build_json_report(&report, std::path::Path::new("inklog_config.toml"));
        assert_eq!(json["command"], "doctor");
        assert_eq!(json["target"], "security");
        assert!(json["checks"].as_array().is_some());
        assert!(json["critical"].is_u64());
        assert!(json["warn"].is_u64());
        assert_eq!(json["sound"], serde_json::json!(report.is_sound()));
        // 逐检查项字段齐全，status 为小写字符串
        for check in json["checks"].as_array().unwrap() {
            assert!(check["id"].is_string());
            assert!(check["capability"].is_string());
            assert!(check["status"].is_string());
        }
    }

    /// 配置文件缺失 → 显性错误（对齐 validate）。
    #[test]
    fn test_missing_config_is_error() {
        let result =
            run_doctor_security(Some(PathBuf::from("/nonexistent/inklog/cfg.toml")), false);
        assert!(result.is_err(), "missing config must be an explicit error");
    }
}
