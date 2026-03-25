// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 断路器实现，用于 Sink 的故障隔离与自动恢复

use std::sync::{Arc, Mutex};
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

/// 断路器实现
#[derive(Debug)]
pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitState>>,
    failure_count: Arc<Mutex<u32>>,
    success_count: Arc<Mutex<u32>>,
    last_failure: Arc<Mutex<Option<Instant>>>,
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
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            failure_count: Arc::new(Mutex::new(0)),
            success_count: Arc::new(Mutex::new(0)),
            last_failure: Arc::new(Mutex::new(None)),
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
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            failure_count: Arc::new(Mutex::new(0)),
            success_count: Arc::new(Mutex::new(0)),
            last_failure: Arc::new(Mutex::new(None)),
            config,
        }
    }

    /// 获取当前状态
    pub fn state(&self) -> CircuitState {
        self.state
            .lock()
            .map(|guard| *guard)
            .unwrap_or(CircuitState::Closed)
    }

    /// 检查是否可以执行操作
    pub fn can_execute(&self) -> bool {
        let state = self
            .state
            .lock()
            .map(|guard| *guard)
            .unwrap_or(CircuitState::Closed);
        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // 检查是否超时
                let last_failure = self.last_failure.lock().ok().and_then(|guard| *guard);
                if let Some(time) = last_failure {
                    if time.elapsed() >= self.config.timeout {
                        // 超时，进入半开状态
                        if let Ok(mut guard) = self.state.lock() {
                            *guard = CircuitState::HalfOpen;
                        }
                        if let Ok(mut guard) = self.success_count.lock() {
                            *guard = 0;
                        }
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// 记录成功
    pub fn record_success(&mut self) {
        if let Ok(mut state_guard) = self.state.lock() {
            if let Ok(mut success_count_guard) = self.success_count.lock() {
                match *state_guard {
                    CircuitState::HalfOpen => {
                        *success_count_guard += 1;
                        if *success_count_guard >= self.config.success_threshold {
                            *state_guard = CircuitState::Closed;
                            if let Ok(mut failure_count_guard) = self.failure_count.lock() {
                                *failure_count_guard = 0;
                            }
                        }
                    }
                    CircuitState::Open => {
                        // 意外的成功，重置
                        *state_guard = CircuitState::Closed;
                        if let Ok(mut failure_count_guard) = self.failure_count.lock() {
                            *failure_count_guard = 0;
                        }
                    }
                    CircuitState::Closed => {
                        // 成功，重置失败计数
                        if let Ok(mut failure_count_guard) = self.failure_count.lock() {
                            *failure_count_guard = 0;
                        }
                    }
                }
            }
        }
    }

    /// 记录失败
    pub fn record_failure(&mut self) {
        if let Ok(mut state_guard) = self.state.lock() {
            if let Ok(mut failure_count_guard) = self.failure_count.lock() {
                if let Ok(mut last_failure_guard) = self.last_failure.lock() {
                    *last_failure_guard = Some(Instant::now());
                    *failure_count_guard += 1;

                    match *state_guard {
                        CircuitState::HalfOpen => {
                            *state_guard = CircuitState::Open;
                        }
                        CircuitState::Closed => {
                            if *failure_count_guard >= self.config.failure_threshold {
                                *state_guard = CircuitState::Open;
                            }
                        }
                        CircuitState::Open => {
                            // 已经是打开状态，更新失败时间
                        }
                    }
                }
            }
        }
    }

    /// 重置断路器到初始状态
    pub fn reset(&mut self) {
        if let Ok(mut guard) = self.state.lock() {
            *guard = CircuitState::Closed;
        }
        if let Ok(mut guard) = self.failure_count.lock() {
            *guard = 0;
        }
        if let Ok(mut guard) = self.success_count.lock() {
            *guard = 0;
        }
        if let Ok(mut guard) = self.last_failure.lock() {
            *guard = None;
        }
    }

    /// 获取失败次数
    pub fn failure_count(&self) -> u32 {
        self.failure_count.lock().map(|guard| *guard).unwrap_or(0)
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
        let mut cb = CircuitBreaker::new(3, StdDuration::from_secs(1), 3);
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
        let mut cb = CircuitBreaker::new(2, StdDuration::from_millis(100), 3);
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
        let mut cb = CircuitBreaker::new(2, StdDuration::from_millis(100), 3);
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
        let mut cb = CircuitBreaker::new(2, StdDuration::from_secs(1), 3);
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
}
