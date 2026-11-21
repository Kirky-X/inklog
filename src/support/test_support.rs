// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 测试辅助：跨模块共享的进程级串行锁探针。
//!
//! 收编自 `domain/config/config.rs` 与 `integrations/infra/config.rs` 中两份逐字的
//! `test_env_mutation_holds_process_wide_serial_lock` 测试体。探针序列与断言文本
//! 逐字保留；`#[test] #[serial]` 属性与测试名留在各调用方的测试 fn 上。

/// 断言当前测试持有进程级串行锁，并执行 `INKLOG_*` env 探针序列。
pub(crate) fn assert_env_mutation_runs_under_serial_lock() {
    assert!(
        serial_test::is_locked_serially(None),
        "env mutation must run under the process-wide serial lock, \
         not a module-local mutex"
    );
    unsafe {
        std::env::set_var("INKLOG_TEST_SERIAL_LOCK_PROBE", "1");
    }
    assert_eq!(
        std::env::var("INKLOG_TEST_SERIAL_LOCK_PROBE").as_deref(),
        Ok("1"),
        "env mutation must be observable while the lock is held"
    );
    unsafe {
        std::env::remove_var("INKLOG_TEST_SERIAL_LOCK_PROBE");
    }
}
