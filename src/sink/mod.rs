// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

pub mod async_file;
pub mod circuit_breaker;
pub mod compression;
pub mod console;
#[cfg(feature = "dbnexus")]
pub mod database;
pub mod encryption;
#[cfg(feature = "dbnexus")]
pub mod entity;
pub mod file;
pub mod ring_buffered_file;

use crate::error::InklogError;
use crate::log_record::LogRecord;

pub trait LogSink: Send + Sync {
    fn write(&mut self, record: &LogRecord) -> Result<(), InklogError>;
    fn flush(&mut self) -> Result<(), InklogError>;
    fn is_healthy(&self) -> bool {
        true
    }
    fn shutdown(&mut self) -> Result<(), InklogError>;

    // 轮转相关方法
    fn start_rotation_timer(&mut self) {
        // 默认空实现
    }

    fn stop_rotation_timer(&mut self) {
        // 默认空实现
    }

    fn check_disk_space(&self) -> Result<bool, InklogError> {
        Ok(true) // 默认返回有足够空间
    }
}
