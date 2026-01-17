// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

pub mod async_file;
pub mod compression;
pub mod console;
pub mod database;
pub mod encryption;
pub mod file;
pub mod ring_buffered_file;

use crate::error::InklogError;
use crate::log_record::LogRecord;
use std::time::{Duration, Instant};

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

/// 断路器状态
#[derive(Debug, Clone, Copy, PartialEq)]
enum CircuitState {
    Closed,   // 正常运行
    Open,     // 开启（停止写入，进入降级）
    HalfOpen, // 半开启（尝试恢复）
}

/// 断路器实现，用于 Sink 的故障隔离与自动恢复
#[derive(Debug)]
struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    failure_threshold: u32,
    reset_timeout: Duration,
    last_failure_time: Option<Instant>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            failure_threshold,
            reset_timeout,
            last_failure_time: None,
        }
    }

    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    if last_failure.elapsed() >= self.reset_timeout {
                        self.state = CircuitState::HalfOpen;
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = CircuitState::Closed;
        self.last_failure_time = None;
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(Instant::now());
        if self.failure_count >= self.failure_threshold {
            self.state = CircuitState::Open;
        }
    }

    pub fn state(&self) -> CircuitState {
        self.state
    }

    #[allow(dead_code)]
    pub fn failure_count(&self) -> u32 {
        self.failure_count
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.failure_count = 0;
        self.last_failure_time = None;
    }
}

#[cfg(test)]
mod circuit_breaker_tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_circuit_breaker_initial_state() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count, 0);
        assert!(cb.can_execute());
    }

    #[test]
    fn test_circuit_breaker_success() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count, 0);
        assert!(cb.can_execute());
    }

    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.can_execute());
    }

    #[test]
    fn test_circuit_breaker_half_open_after_timeout() {
        let mut cb = CircuitBreaker::new(3, Duration::from_millis(100));

        // Open the circuit
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Wait for timeout (longer than reset_timeout)
        std::thread::sleep(Duration::from_millis(200));

        // can_execute should transition to HalfOpen and return true
        let result = cb.can_execute();
        assert!(result, "can_execute should return true in HalfOpen state");
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_circuit_breaker_success_resets_state() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));

        // Record some failures
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        // Record success
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count, 0);
        assert!(cb.last_failure_time.is_none());
    }

    #[test]
    fn test_circuit_breaker_half_open_success() {
        let mut cb = CircuitBreaker::new(3, Duration::from_millis(100));

        // Open the circuit
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(200));

        // Call can_execute to trigger HalfOpen transition
        let _ = cb.can_execute();

        // Should be HalfOpen
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Success should close the circuit
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_half_open_failure() {
        let mut cb = CircuitBreaker::new(3, Duration::from_millis(100));

        // Open the circuit
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(200));

        // Call can_execute to trigger HalfOpen transition
        let _ = cb.can_execute();

        // Should be HalfOpen
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Failure should open the circuit again
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));

        // Open the circuit
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Reset
        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count, 0);
        assert!(cb.last_failure_time.is_none());
    }

    #[test]
    fn test_circuit_breaker_opens_on_exact_threshold() {
        let mut cb = CircuitBreaker::new(2, Duration::from_secs(30));
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_multiple_failures_beyond_threshold() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }
}
