// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! `inklog-cli` 入口薄壳：clap 定义与子命令分发在 [`cli`] 模块
//! （`src/cli/mod.rs`），本文件只保留进程入口与退出码契约。

mod cli;

fn main() {
    // 退出码契约：0 = 成功/有匹配，1 = 错误，2 = 无匹配/校验失败
    let code = cli::run_cli();
    if code != 0 {
        std::process::exit(code);
    }
}

// 测试进程固定 locale 为 en：与 CI（Linux，LANG 未设）行为一致。
// bin 测试链接的是非 test 编译的 lib，lib 内的 cfg(test) 分支在此
// 不生效；ctor 于 main 之前设置文档化最高优先级 override
// （INKLOG_LOCALE，见 i18n/locale_manager.rs），对全进程测试生效。
#[cfg(test)]
mod test_locale_init {
    #[ctor::ctor(unsafe)]
    fn set_test_locale() {
        // 测试进程启动期（单线程、无其他线程读 env），set_var 无 UB 风险
        unsafe {
            std::env::set_var("INKLOG_LOCALE", "en");
        }
    }
}
