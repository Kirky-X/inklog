// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 金样本导出器（切换验证一次性工具，验证完即删）。
//!
//! 驱动现网 inklog::i18n 公共面（tr/tr_args），对全部内嵌 (locale, key)
//! 组合在 NOARG 与统一超集参数两种形态下渲染，产出 TSV：
//! `<locale>\t<key>\t<ARG|NOARG>\t<rendered>`。
//! 渲染文本中的 `\t`、`\n`、`\r` 转义保单行。locale 经 INKLOG_LOCALE
//! 每次进程启动切换（二进程法：同参数跑 en 与 zh-CN 各一次，键序一致）。

use std::collections::BTreeMap;

const SUPERSET: &[(&str, &str)] = &[
    ("path", "/tmp/p"),
    ("level", "INFO"),
    ("len", "42"),
    ("enabled", "true"),
    ("count", "7"),
    ("size", "100MB"),
    ("env", "KEY"),
    ("n", "3"),
    ("driver", "sqlite"),
    ("valid", "a,b"),
    ("section", "db"),
    ("deps", "rust"),
    ("error", "E"),
    ("err", "E2"),
    ("reason", "R"),
    ("dir", "/tmp/d"),
    ("msg", "M"),
    ("name", "N"),
    ("value", "42"),
    ("code", "C"),
    ("field", "F"),
    ("sentinel", "S"),
    ("message", "MSG"),
    ("detail", "D"),
    ("input", "I"),
    ("output", "O"),
    ("key", "K"),
    ("id", "1"),
    ("column", "col"),
    ("table", "tbl"),
    ("version", "v1"),
    ("state", "ST"),
    ("status", "200"),
    ("type", "T"),
    ("time", "12:00"),
    ("date", "2026-01-01"),
    ("duration", "1s"),
    ("limit", "10"),
    ("max", "100"),
    ("min", "1"),
    ("url", "http://x"),
    ("port", "8080"),
    ("host", "h"),
    ("user", "u"),
    ("file", "f.log"),
    ("bytes", "9"),
    ("seconds", "5"),
    ("timeout", "30"),
    ("capacity", "64"),
    ("threads", "4"),
    ("masked", "X"),
    ("content", "CT"),
    ("log_message", "LM"),
    ("masked_log", "ML"),
    ("throughput", "9.5"),
    ("marker", "*"),
    ("level_marker", ">"),
];

/// FTL 键提取：与 catalog 全键集同口径（键起始行；多行值由 tr 引擎自己解析，
/// 这里只负责枚举键）。文件顺序与 locale_real 的 EMBEDDED_LOCALES 一致。
const FTL_FILES: &[&str] = &[
    "cli.ftl",
    "config.ftl",
    "error.ftl",
    "log_level.ftl",
    "sink.ftl",
    "metrics.ftl",
    "validation.ftl",
    "examples.ftl",
];

fn main() {
    let locale = std::env::var("INKLOG_LOCALE").unwrap_or_else(|_| "en".into());
    let out_path = std::env::var("INKLOG_GOLDEN_OUT").unwrap_or_else(|_| {
        format!("/tmp/idle-remediation/work/inklog-golden/golden-{locale}.tsv")
    });

    // 键序：EMBEDDED 文件序 + 行序（两侧一致，diff 可逐行对齐）
    let mut keys: Vec<String> = Vec::new();
    let mut seen: BTreeMap<String, ()> = BTreeMap::new();
    for name in FTL_FILES {
        let path = format!("locales/{locale}/{name}");
        let source = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        for line in source.lines() {
            if line.is_empty() || line.starts_with('#') || line.starts_with(char::is_whitespace) {
                continue;
            }
            if let Some((k, _)) = line.split_once('=') {
                let key = k.trim();
                if key.is_empty() || key.starts_with('-') || key.starts_with('.') {
                    continue;
                }
                if seen.insert(key.to_string(), ()).is_none() {
                    keys.push(key.to_string());
                }
            }
        }
    }

    let mut lines: Vec<String> = Vec::new();
    for key in &keys {
        // NOARG：tr 全局惰性初始化在本进程首调用时按 INKLOG_LOCALE 定格
        let noarg = inklog::i18n::tr(key);
        lines.push(format!("{locale}\t{key}\tNOARG\t{}", esc(&noarg)));
        let mut args = inklog::i18n::MsgArgs::new();
        for (k, v) in SUPERSET {
            args.set(k, v);
        }
        let arg = inklog::i18n::tr_args(key, args);
        lines.push(format!("{locale}\t{key}\tARG\t{}", esc(&arg)));
    }

    std::fs::write(&out_path, lines.join("\n") + "\n")
        .unwrap_or_else(|e| panic!("write {out_path}: {e}"));
    eprintln!("golden lines: {} → {out_path}", lines.len());
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
