// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 断路器实现，用于 Sink 的故障隔离与自动恢复

use parking_lot::Mutex;
use std::time::{Duration as StdDuration, Instant};

/// 断路器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// 正常状态
    Closed,
    /// 故障状态，请求快速失败
    Open,
    /// 半开状态，尝试恢复
    HalfOpen,
}

/// 断路器配置
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// 失败次数阈值
    pub failure_threshold: u32,
    /// 半开状态下成功次数阈值
    pub success_threshold: u32,
    /// 超时时间
    pub timeout: StdDuration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: StdDuration::from_secs(30),
        }
    }
}

/// Internal state protected by a single lock.
#[derive(Debug)]
struct CircuitBreakerInner {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure: Option<Instant>,
}

/// 断路器实现
///
/// Uses a single `parking_lot::Mutex` to protect all mutable state,
/// reducing lock acquisitions from 4 to 1 per operation.
#[derive(Debug)]
pub struct CircuitBreaker {
    inner: Mutex<CircuitBreakerInner>,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    /// 创建新的断路器
    ///
    /// # Arguments
    /// * `failure_threshold` - 失败次数阈值
    /// * `timeout` - 超时时间
    /// * `success_threshold` - 半开状态下成功次数阈值
    pub fn new(failure_threshold: u32, timeout: StdDuration, success_threshold: u32) -> Self {
        Self {
            inner: Mutex::new(CircuitBreakerInner {
                state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure: None,
            }),
            config: CircuitBreakerConfig {
                failure_threshold,
                success_threshold,
                timeout,
            },
        }
    }

    /// 使用配置创建新的断路器
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            inner: Mutex::new(CircuitBreakerInner {
                state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure: None,
            }),
            config,
        }
    }

    /// 获取当前状态
    pub fn state(&self) -> CircuitState {
        self.inner.lock().state
    }

    /// Check whether an operation may proceed.
    ///
    /// # Side Effect
    ///
    /// When the current state is `Open` and the timeout has elapsed since the
    /// last recorded failure, this method **transitions** the state to `HalfOpen`
    /// and resets `success_count` to 0.  Callers should be aware that repeated
    /// invocations in the timeout window are not pure reads — the first call
    /// after the timeout will mutate the internal state.
    pub fn can_execute(&self) -> bool {
        let mut inner = self.inner.lock();
        match inner.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // 检查是否超时
                if let Some(time) = inner.last_failure
                    && time.elapsed() >= self.config.timeout
                {
                    // 超时，进入半开状态
                    inner.state = CircuitState::HalfOpen;
                    inner.success_count = 0;
                    return true;
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// 记录成功
    pub fn record_success(&self) {
        let mut inner = self.inner.lock();
        match inner.state {
            CircuitState::HalfOpen => {
                inner.success_count += 1;
                if inner.success_count >= self.config.success_threshold {
                    inner.state = CircuitState::Closed;
                    inner.failure_count = 0;
                }
            }
            CircuitState::Open => {
                // 意外的成功，重置
                inner.state = CircuitState::Closed;
                inner.failure_count = 0;
            }
            CircuitState::Closed => {
                // 成功，重置失败计数
                inner.failure_count = 0;
            }
        }
    }

    /// 记录失败
    pub fn record_failure(&self) {
        let mut inner = self.inner.lock();
        inner.last_failure = Some(Instant::now());
        inner.failure_count += 1;

        match inner.state {
            CircuitState::HalfOpen => {
                inner.state = CircuitState::Open;
            }
            CircuitState::Closed => {
                if inner.failure_count >= self.config.failure_threshold {
                    inner.state = CircuitState::Open;
                }
            }
            CircuitState::Open => {
                // 已经是打开状态，更新失败时间
            }
        }
    }

    /// 重置断路器到初始状态
    pub fn reset(&self) {
        let mut inner = self.inner.lock();
        inner.state = CircuitState::Closed;
        inner.failure_count = 0;
        inner.success_count = 0;
        inner.last_failure = None;
    }

    /// 获取失败次数
    pub fn failure_count(&self) -> u32 {
        self.inner.lock().failure_count
    }

    /// 获取配置
    pub fn config(&self) -> &CircuitBreakerConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_initial_state() {
        let cb = CircuitBreaker::new(3, StdDuration::from_secs(1), 3);
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
        assert!(cb.can_execute());
    }

    #[test]
    fn test_circuit_breaker_open_after_failures() {
        let cb = CircuitBreaker::new(3, StdDuration::from_secs(1), 3);
        assert!(cb.can_execute());

        cb.record_failure();
        assert!(cb.can_execute());
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        assert!(cb.can_execute());

        cb.record_failure();
        assert!(!cb.can_execute());
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_half_open_after_timeout() {
        let cb = CircuitBreaker::new(2, StdDuration::from_millis(100), 3);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.can_execute());

        std::thread::sleep(std::time::Duration::from_millis(150));
        assert!(cb.can_execute());
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_circuit_breaker_close_after_successes() {
        let cb = CircuitBreaker::new(2, StdDuration::from_millis(100), 3);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        std::thread::sleep(std::time::Duration::from_millis(150));
        // Must call can_execute() to trigger Open -> HalfOpen transition
        assert!(cb.can_execute());
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // First success - still HalfOpen
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Second success - still HalfOpen (need 3 total)
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Third success - reaches threshold, closes
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let cb = CircuitBreaker::new(2, StdDuration::from_secs(1), 3);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
        assert!(cb.can_execute());
    }

    #[test]
    fn test_circuit_breaker_with_config() {
        let config = CircuitBreakerConfig {
            failure_threshold: 10,
            success_threshold: 5,
            timeout: StdDuration::from_secs(60),
        };
        let cb = CircuitBreaker::with_config(config.clone());
        assert_eq!(cb.config().failure_threshold, 10);
        assert_eq!(cb.config().success_threshold, 5);
    }

    #[test]
    fn test_circuit_breaker_config_default() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 3);
        assert_eq!(config.timeout, StdDuration::from_secs(30));
    }

    #[test]
    fn test_record_success_on_open_state() {
        // Test the unexpected success on Open state - should reset to Closed
        let cb = CircuitBreaker::new(2, StdDuration::from_secs(60), 3);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // record_success on Open state should reset to Closed
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_record_failure_on_half_open_state() {
        // Test failure on HalfOpen state - should transition back to Open
        let cb = CircuitBreaker::new(2, StdDuration::from_millis(100), 3);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        std::thread::sleep(std::time::Duration::from_millis(150));
        assert!(cb.can_execute()); // triggers transition to HalfOpen
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // record_failure on HalfOpen should transition back to Open
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_record_failure_on_open_state() {
        // Test failure on Open state - should stay Open and update failure time
        let cb = CircuitBreaker::new(2, StdDuration::from_secs(60), 3);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert_eq!(cb.failure_count(), 2);

        // record_failure on Open state should stay Open
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert_eq!(cb.failure_count(), 3);
    }

    #[test]
    fn test_record_success_on_closed_state() {
        // Test success on Closed state - should reset failure count
        let cb = CircuitBreaker::new(3, StdDuration::from_secs(60), 3);
        cb.record_failure();
        assert_eq!(cb.failure_count(), 1);

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_half_open_can_execute() {
        // Verify HalfOpen state allows execution
        let cb = CircuitBreaker::new(1, StdDuration::from_millis(50), 2);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(cb.can_execute()); // triggers transition to HalfOpen
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // In HalfOpen state, can_execute should return true
        assert!(cb.can_execute());
    }

    #[test]
    fn test_open_state_can_execute_before_timeout() {
        // In Open state before timeout, can_execute should return false
        let cb = CircuitBreaker::new(1, StdDuration::from_secs(60), 2);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.can_execute());
    }
}
