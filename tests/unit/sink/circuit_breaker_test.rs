// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

// 熔断器功能测试
// 测试熔断器模式在外部服务故障时的保护机制

#[cfg(test)]
mod circuit_breaker_test {
    use inklog::sink::circuit_breaker::{CircuitBreaker, CircuitBreakerState};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;
    use tokio::time::timeout;

    // === 熔断器状态测试 ===

    #[test]
    fn test_circuit_breaker_initial_state() {
        let breaker = CircuitBreaker::new(5, Duration::from_secs(60));
        
        assert_eq!(breaker.state(), CircuitBreakerState::Closed);
        assert_eq!(breaker.failure_count(), 0);
        assert_eq!(breaker.success_count(), 0);
    }

    #[test]
    fn test_circuit_breaker_state_transitions() {
        let breaker = CircuitBreaker::new(3, Duration::from_secs(60));
        
        // 初始状态
        assert_eq!(breaker.state(), CircuitBreakerState::Closed);
        
        // 达到失败阈值，进入打开状态
        for _ in 0..3 {
            let result: Result<(), ()> = Err(());
            breaker.record_result(result);
        }
        
        assert_eq!(breaker.state(), CircuitBreakerState::Open);
        
        // 半开状态转换（超时后）
        // 注意：半开状态转换由定时器控制，这里不测试
    }

    #[test]
    fn test_circuit_breaker_failure_counting() {
        let breaker = CircuitBreaker::new(5, Duration::from_secs(60));
        
        assert_eq!(breaker.failure_count(), 0);
        
        // 记录失败
        for i in 1..=3 {
            let result: Result<(), ()> = Err(());
            breaker.record_result(result);
            assert_eq!(breaker.failure_count(), i);
        }
    }

    #[test]
    fn test_circuit_breaker_success_counting() {
        let breaker = CircuitBreaker::new(5, Duration::from_secs(60));
        
        assert_eq!(breaker.success_count(), 0);
        
        // 记录成功
        for i in 1..=3 {
            let result: Result<(), ()> = Ok(());
            breaker.record_result(result);
            assert_eq!(breaker.success_count(), i);
        }
    }

    #[test]
    fn test_circuit_breaker_reset_on_success() {
        let breaker = CircuitBreaker::new(3, Duration::from_secs(60));
        
        // 记录一些失败
        for _ in 0..2 {
            let result: Result<(), ()> = Err(());
            breaker.record_result(result);
        }
        assert_eq!(breaker.failure_count(), 2);
        
        // 记录成功应该重置失败计数
        let result: Result<(), ()> = Ok(());
        breaker.record_result(result);
        assert_eq!(breaker.failure_count(), 0);
    }

    // === 熔断器保护测试 ===

    #[tokio::test]
    async fn test_circuit_breaker_allows_calls_in_closed_state() {
        let breaker = Arc::new(Mutex::new(CircuitBreaker::new(3, Duration::from_secs(60))));
        let call_count = Arc::new(AtomicUsize::new(0));
        
        // 在闭合状态下，应该允许调用
        for _ in 0..5 {
            let guard = breaker.lock().await;
            assert!(guard.allow_request());
            drop(guard);
            
            let count = call_count.clone();
            let result = async {
                let mut guard = breaker.lock().await;
                guard.record_result(Ok(()));
                count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }.await;
            
            assert!(result.is_ok());
        }
        
        assert_eq!(call_count.load(Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn test_circuit_breaker_blocks_calls_in_open_state() {
        let breaker = Arc::new(Mutex::new(CircuitBreaker::new(2, Duration::from_secs(60))));
        
        // 触发打开状态
        for _ in 0..2 {
            let mut guard = breaker.lock().await;
            guard.record_result(Err(()));
        }
        
        // 在打开状态下，应该拒绝调用
        {
            let guard = breaker.lock().await;
            assert!(!guard.allow_request());
        }
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_state() {
        let breaker = Arc::new(Mutex::new(CircuitBreaker::new(1, Duration::from_millis(50))));
        
        // 触发打开状态
        {
            let mut guard = breaker.lock().await;
            guard.record_result(Err(()));
        }
        
        // 等待半开转换
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // 半开状态应该允许一个测试调用
        {
            let guard = breaker.lock().await;
            assert_eq!(guard.state(), CircuitBreakerState::HalfOpen);
            assert!(guard.allow_request());
        }
    }

    // === 熔断器配置测试 ===

    #[test]
    fn test_circuit_breaker_config() {
        let breaker = CircuitBreaker::new(10, Duration::from_secs(300));
        
        assert_eq!(breaker.failure_threshold(), 10);
        assert_eq!(breaker.recovery_timeout(), Duration::from_secs(300));
    }

    #[test]
    fn test_circuit_breaker_invalid_config() {
        // 阈值应该至少为 1
        let breaker = CircuitBreaker::new(1, Duration::from_secs(60));
        assert_eq!(breaker.failure_threshold(), 1);
    }

    // === 熔断器并发测试 ===

    #[tokio::test]
    async fn test_circuit_breaker_concurrent_calls() {
        let breaker = Arc::new(Mutex::new(CircuitBreaker::new(100, Duration::from_secs(60))));
        let success_count = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(tokio::sync::Barrier::new(20));
        
        let handles: Vec<_> = (0..20)
            .map(|_| {
                let barrier = barrier.clone();
                let breaker = breaker.clone();
                let success_count = success_count.clone();
                tokio::spawn(async move {
                    barrier.wait().await;
                    for _ in 0..5 {
                        let guard = breaker.lock().await;
                        if guard.allow_request() {
                            drop(guard);
                            let mut guard = breaker.lock().await;
                            guard.record_result(Ok(()));
                            success_count.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                })
            })
            .collect();
        
        for handle in handles {
            handle.await.unwrap();
        }
        
        // 所有调用都应该成功
        assert_eq!(success_count.load(Ordering::SeqCst), 100);
    }

    // === 熔断器错误处理测试 ===

    #[tokio::test]
    async fn test_circuit_breaker_error_types() {
        let breaker = Arc::new(Mutex::new(CircuitBreaker::new(2, Duration::from_secs(60))));
        
        // 不同类型的错误都应该被记录为失败
        let errors = [
            Err::<(), &str>("connection refused"),
            Err::<(), &str>("timeout"),
            Err::<(), &str>("service unavailable"),
        ];
        
        for error in errors {
            let mut guard = breaker.lock().await;
            guard.record_result(error.map_err(|e| e.to_string()));
        }
        
        assert_eq!(breaker.state(), CircuitBreakerState::Open);
    }

    // === 熔断器恢复测试 ===

    #[tokio::test]
    async fn test_circuit_breaker_recovery_on_success() {
        let breaker = Arc::new(Mutex::new(CircuitBreaker::new(2, Duration::from_millis(50))));
        
        // 触发打开状态
        {
            let mut guard = breaker.lock().await;
            guard.record_result(Err(()));
            guard.record_result(Err(()));
        }
        
        // 等待半开状态
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // 在半开状态记录成功应该关闭熔断器
        {
            let mut guard = breaker.lock().await;
            assert_eq!(guard.state(), CircuitBreakerState::HalfOpen);
            guard.record_result(Ok(()));
            // 半开状态下成功应该转换回闭合
        }
        
        // 验证恢复
        tokio::time::sleep(Duration::from_millis(10)).await;
        let guard = breaker.lock().await;
        assert_eq!(guard.state(), CircuitBreakerState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reopen_on_failure() {
        let breaker = Arc::new(Mutex::new(CircuitBreaker::new(2, Duration::from_millis(50))));
        
        // 触发打开状态
        {
            let mut guard = breaker.lock().await;
            guard.record_result(Err(()));
            guard.record_result(Err(()));
        }
        
        // 等待半开状态
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // 在半开状态记录失败应该重新打开
        {
            let mut guard = breaker.lock().await;
            assert_eq!(guard.state(), CircuitBreakerState::HalfOpen);
            guard.record_result(Err(()));
        }
        
        // 验证重新打开
        tokio::time::sleep(Duration::from_millis(10)).await;
        let guard = breaker.lock().await;
        assert_eq!(guard.state(), CircuitBreakerState::Open);
    }
}
