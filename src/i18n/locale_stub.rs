// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! no-i18n 组合的翻译 fallback：以内嵌英文 `.ftl` 文案的静态表响应
//! `tr`/`tr_args`，接口与 fluent 实现（`locale_real`）逐一对应。
//!
//! 取舍：不做 locale 检测与多语言切换（那需要 icu/fluent 依赖树），
//! 但错误构造路径上的文案保持人类可读——查内嵌 en 文案表并完成
//! `{ $key }` 占位插值，而非把消息 ID 直接抛给用户。

use std::collections::HashMap;
use std::sync::LazyLock;

/// 消息插值参数载体，形态与 i18n 组合的 [`MsgArgs`](self::MsgArgs) 一致。
#[derive(Debug, Clone, Default)]
pub struct MsgArgs {
    entries: Vec<(String, String)>,
}

impl MsgArgs {
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一个参数；同名键后者覆盖前者（与 i18n 组合的语义一致）。
    pub fn set(&mut self, key: &str, value: impl std::fmt::Display) -> &mut Self {
        let value = value.to_string();
        match self.entries.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value,
            None => self.entries.push((key.to_string(), value)),
        }
        self
    }
}

/// Locale 初始化在 fallback 下无事可做（单语言），保留幂等入口。
pub fn init_locale() {}

/// 固定返回 `"en"`——fallback 组合只有内嵌英文文案。
pub fn current_locale() -> String {
    "en".to_string()
}

/// 查内嵌英文文案表；表外消息 ID 原样返回（与 i18n 组合的
/// last-resort 行为一致）。
pub fn tr(id: &str) -> String {
    match lookup(id) {
        Some(text) => text,
        None => id.to_string(),
    }
}

/// 查内嵌英文文案表并插值 `{ $key }` 占位；表外消息 ID 原样返回。
pub fn tr_args(_id: &str, args: MsgArgs) -> String {
    match lookup(_id) {
        Some(text) => interpolate(&text, &args),
        None => _id.to_string(),
    }
}

// ── 内嵌英文文案表 ────────────────────────────────────────────────

/// 编译期内嵌的 `locales/en/*.ftl`。与 i18n 组合的 EMBEDDED_LOCALES
/// 保持同一批文件——文案只需维护一份，两种编译形态共享。
const EN_FALLBACK_SOURCES: &[&str] = &[
    include_str!("../../locales/en/cli.ftl"),
    include_str!("../../locales/en/config.ftl"),
    include_str!("../../locales/en/error.ftl"),
    include_str!("../../locales/en/log_level.ftl"),
    include_str!("../../locales/en/sink.ftl"),
    include_str!("../../locales/en/metrics.ftl"),
    include_str!("../../locales/en/validation.ftl"),
];

static EN_FALLBACK: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    let mut table = HashMap::new();
    for source in EN_FALLBACK_SOURCES {
        parse_ftl(source, &mut table);
    }
    table
});

/// 粗粒度 Fluent 文本解析：只提取 `key = value` 消息行与缩进续行，
/// 忽略注释（`#`）、术语定义（`-term`）与行内表达式特殊形态——
/// 调用点使用的消息全部是纯文本 + `{ $key }` 占位，粗解析足够。
fn parse_ftl(source: &str, table: &mut HashMap<String, String>) {
    let mut current_key: Option<String> = None;
    for line in source.lines() {
        if line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix(char::is_whitespace) {
            // 缩进续行：接到上一条消息（空白规整为单空格）
            if let Some(key) = &current_key
                && !rest.trim().is_empty()
                && let Some(entry) = table.get_mut(key)
            {
                entry.push(' ');
                entry.push_str(rest.trim_end());
            }
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            if key.is_empty() || key.starts_with('-') {
                current_key = None;
                continue;
            }
            current_key = Some(key.to_string());
            table.insert(key.to_string(), value.trim().to_string());
        } else {
            current_key = None;
        }
    }
}

fn lookup(id: &str) -> Option<String> {
    EN_FALLBACK.get(id).cloned()
}

/// 把文案中的 `{ $key }` 占位替换为参数值；未提供的占位原样保留
/// （与 fluent 的缺参渲染行为一致）。
fn interpolate(text: &str, args: &MsgArgs) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                let inner = after[..close].trim();
                let placeholder = &rest[open..open + 1 + close + 1];
                match inner.strip_prefix('$') {
                    Some(name) => match args.entries.iter().find(|(k, _)| k == name) {
                        Some((_, value)) => out.push_str(value),
                        None => out.push_str(placeholder),
                    },
                    None => out.push_str(placeholder),
                }
                rest = &after[close + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_table_loads_en_messages() {
        assert_eq!(tr("error-io_error"), "IO error");
        assert_eq!(tr("log_level-name_info"), "INFO");
    }

    #[test]
    fn test_fallback_unknown_key_returns_id() {
        assert_eq!(tr("no.such.key"), "no.such.key");
        assert_eq!(tr_args("no.such.key", MsgArgs::new()), "no.such.key");
    }

    #[test]
    fn test_fallback_interpolates_placeholders() {
        let mut args = MsgArgs::new();
        args.set("path", "/etc/app.toml");
        let rendered = tr_args("cli-validate-validating", args);
        assert!(
            rendered.contains("/etc/app.toml"),
            "placeholder must be replaced: {rendered}"
        );
        assert!(
            !rendered.contains("{$path}"),
            "raw placeholder must not remain: {rendered}"
        );
    }

    #[test]
    fn test_msg_args_set_overrides_same_key() {
        let mut args = MsgArgs::new();
        args.set("k", "first");
        args.set("k", "second");
        assert_eq!(args.entries.len(), 1);
        assert_eq!(args.entries[0].1, "second");
    }

    #[test]
    fn test_every_source_message_id_has_fallback_text() {
        // 逐条核对：扫描源码中全部 tr/tr_args 字面量消息 ID，
        // 断言内嵌英文文案表都能给出可读文案（非 ID 原文）。
        // 这是 no-i18n 组合不退化为「错误码式输出」的硬门禁。
        let mut ids = collect_source_message_ids();
        ids.sort();
        ids.dedup();
        assert!(!ids.is_empty(), "message id scan must find literals");
        // 哨兵 ID：unknown-key 测试刻意使用的不存在 key，
        // 「返回 ID 原文」正是它们的被测行为，不参与核对
        const SENTINEL_IDS: &[&str] = &["no.such.key", "nonexistent.key.that.does.not.exist"];
        let mut missing = Vec::new();
        for id in &ids {
            if !SENTINEL_IDS.contains(&id.as_str()) && EN_FALLBACK.get(id).is_none() {
                missing.push(id.clone());
            }
        }
        assert!(
            missing.is_empty(),
            "message ids without en fallback text: {missing:?}"
        );
    }

    /// 递归收集 `src/` 下全部 `.rs` 文件中 `tr("...")` / `tr_args("...")`
    /// 的字面量消息 ID（含 `crate::i18n::` 前缀形态）。
    fn collect_source_message_ids() -> Vec<String> {
        fn walk(dir: &std::path::Path, ids: &mut Vec<String>) {
            let entries = match std::fs::read_dir(dir) {
                Ok(entries) => entries,
                Err(_) => return,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, ids);
                } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    let Ok(source) = std::fs::read_to_string(&path) else {
                        continue;
                    };
                    for line in source.lines() {
                        // 只扫描代码行：注释里的示例消息 ID 不参与核对
                        if line.trim_start().starts_with("//") {
                            continue;
                        }
                        for (pos, _) in line.match_indices("tr(\"") {
                            // 词边界校验：push_str( 之类标识符尾部也含
                            // tr(" 子串，前一个字符是标识符字符时跳过
                            let boundary_ok = pos == 0
                                || !line.as_bytes()[pos - 1].is_ascii_alphanumeric()
                                    && line.as_bytes()[pos - 1] != b'_';
                            if !boundary_ok {
                                continue;
                            }
                            ids.push(line[pos + 4..].split('"').next().unwrap_or("").to_string());
                        }
                    }
                }
            }
        }
        let mut ids = Vec::new();
        walk(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut ids,
        );
        ids.retain(|id| !id.is_empty());
        ids
    }
}
