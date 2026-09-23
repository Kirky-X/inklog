// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

mod cli_impl;
mod decrypt;
mod generate;
mod query;
mod validate;

pub use cli_impl::run_cli;

// clap about/help 文案经 i18n 动态生成（tr() 惰性初始化，跟随
// INKLOG_LOCALE / 系统语言检测；en 与未支持语言回退英文）。
// 使用 doc comment 形式会被 clap 当作 help 文案，故说明一律用普通注释。
#[derive(Parser, Debug)]
#[command(name = "inklog")]
#[command(author = "Kirky.X")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(
    about = inklog::i18n::tr("cli-about"),
    long_about = None
)]
struct Cli {
    // 机器可读 JSON 输出（全子命令）
    #[arg(long, global = true)]
    #[arg(help = inklog::i18n::tr("cli-help-json"))]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(name = "decrypt")]
    #[command(about = inklog::i18n::tr("cli-decrypt-about"))]
    Decrypt {
        #[arg(short, long)]
        #[arg(help = inklog::i18n::tr("cli-decrypt-help-input"))]
        input: PathBuf,

        #[arg(short, long)]
        #[arg(help = inklog::i18n::tr("cli-decrypt-help-output"))]
        output: Option<PathBuf>,

        #[arg(short, long, env = "INKLOG_DECRYPT_KEY")]
        #[arg(help = inklog::i18n::tr("cli-decrypt-help-key-env"))]
        #[arg(value_parser = clap::builder::NonEmptyStringValueParser::new())]
        key_env: String,

        #[arg(long)]
        #[arg(help = inklog::i18n::tr("cli-decrypt-help-recursive"))]
        recursive: bool,

        #[arg(long)]
        #[arg(help = inklog::i18n::tr("cli-decrypt-help-batch"))]
        batch: bool,
    },

    #[command(name = "generate")]
    #[command(about = inklog::i18n::tr("cli-generate-about"))]
    Generate {
        #[arg(short, long)]
        #[arg(help = inklog::i18n::tr("cli-generate-help-output"))]
        output: Option<PathBuf>,

        #[arg(short, long)]
        #[arg(help = inklog::i18n::tr("cli-generate-help-config-type"))]
        #[arg(default_value = "full")]
        config_type: ConfigType,

        #[arg(long)]
        #[arg(help = inklog::i18n::tr("cli-generate-help-env-example"))]
        env_example: bool,
    },

    #[command(name = "validate")]
    #[command(about = inklog::i18n::tr("cli-validate-about"))]
    Validate {
        #[arg(short, long)]
        #[arg(help = inklog::i18n::tr("cli-validate-help-config"))]
        config: Option<PathBuf>,

        #[arg(long)]
        #[arg(help = inklog::i18n::tr("cli-validate-help-prerequisites"))]
        prerequisites: bool,
    },

    #[command(name = "query")]
    #[command(about = inklog::i18n::tr("cli-query-about"))]
    Query {
        #[arg(long = "path")]
        #[arg(help = inklog::i18n::tr("cli-query-help-path"))]
        path: Vec<PathBuf>,

        #[arg(long)]
        #[arg(help = inklog::i18n::tr("cli-query-help-since"))]
        since: Option<String>,

        #[arg(long)]
        #[arg(help = inklog::i18n::tr("cli-query-help-until"))]
        until: Option<String>,

        #[arg(long)]
        #[arg(help = inklog::i18n::tr("cli-query-help-level"))]
        level: Option<String>,

        #[arg(long)]
        #[arg(help = inklog::i18n::tr("cli-query-help-grep"))]
        grep: Option<String>,

        #[arg(long, default_value = "1000")]
        #[arg(help = inklog::i18n::tr("cli-query-help-limit"))]
        limit: usize,

        #[arg(long, env = "INKLOG_DECRYPT_KEY")]
        #[arg(help = inklog::i18n::tr("cli-query-help-key-env"))]
        #[arg(value_parser = clap::builder::NonEmptyStringValueParser::new())]
        key_env: Option<String>,
    },

    #[command(name = "verify-chain")]
    #[command(about = inklog::i18n::tr("cli-verify-chain-about"))]
    VerifyChain {
        /// 审计链 manifest（`<stem>.chain.jsonl`）
        #[arg(long = "manifest")]
        #[arg(help = inklog::i18n::tr("cli-verify-chain-help-manifest"))]
        manifest: PathBuf,

        #[arg(long, env = "INKLOG_AUDIT_KEY")]
        #[arg(help = inklog::i18n::tr("cli-verify-chain-help-key-env"))]
        #[arg(value_parser = clap::builder::NonEmptyStringValueParser::new())]
        key_env: String,
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
    // 退出码契约：0 = 成功/有匹配，1 = 错误，2 = 无匹配/校验失败
    let code = run_cli();
    if code != 0 {
        std::process::exit(code);
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
