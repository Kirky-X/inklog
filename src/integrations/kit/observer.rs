// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Build observer for inklog's trait-kit module construction pipeline.
//!
//! `InklogBuildObserver` implements trait-kit's `BuildObserver` trait to
//! log module build events via `tracing`. Register it with
//! `AsyncKit::with_observer()` to get visibility into the DI build process.

use std::time::Duration;

use trait_kit::prelude::BuildObserver;

/// Observes inklog module build pipeline events.
///
/// Logs `tracing::debug!` messages for each build phase:
/// - `on_module_start`: module build started
/// - `on_module_built`: module build completed (with elapsed time)
/// - `on_build_error`: module build failed (with error details)
///
/// # Example
///
/// ```rust,ignore
/// use inklog::integrations::kit::InklogBuildObserver;
/// use trait_kit::prelude::*;
///
/// let mut kit = AsyncKit::new();
/// kit.with_observer(InklogBuildObserver);
/// kit.register::<InklogModule>().unwrap();
/// let built = kit.build().await.unwrap();
/// ```
pub struct InklogBuildObserver;

impl BuildObserver for InklogBuildObserver {
    fn on_module_start(&self, module_name: &'static str) {
        tracing::debug!(
            module = module_name,
            "inklog build observer: module build started"
        );
    }

    fn on_module_built(&self, module_name: &'static str, elapsed: Duration) {
        tracing::debug!(
            module = module_name,
            elapsed_ms = elapsed.as_millis() as u64,
            "inklog build observer: module built successfully"
        );
    }

    fn on_build_error(&self, module_name: &'static str, error: &trait_kit::TraitKitError) {
        tracing::error!(
            module = module_name,
            error = %error,
            "inklog build observer: module build failed"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Verify InklogBuildObserver implements BuildObserver + Send + Sync.
    #[test]
    fn inklog_build_observer_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<InklogBuildObserver>();
    }

    /// on_module_start does not panic.
    #[test]
    fn on_module_start_does_not_panic() {
        let obs = InklogBuildObserver;
        obs.on_module_start("test-module");
    }

    /// on_module_built does not panic.
    #[test]
    fn on_module_built_does_not_panic() {
        let obs = InklogBuildObserver;
        obs.on_module_built("test-module", Duration::from_millis(42));
    }

    /// on_build_error does not panic.
    #[test]
    fn on_build_error_does_not_panic() {
        let obs = InklogBuildObserver;
        let err = trait_kit::TraitKitError::MissingCapability {
            key: "test".to_string(),
        };
        obs.on_build_error("test-module", &err);
    }

    /// Counting observer via dyn dispatch — verifies trait object safety.
    #[test]
    fn observer_dyn_dispatch() {
        let start_count = Arc::new(AtomicUsize::new(0));
        let built_count = Arc::new(AtomicUsize::new(0));

        struct Counting {
            start: Arc<AtomicUsize>,
            built: Arc<AtomicUsize>,
        }

        impl BuildObserver for Counting {
            fn on_module_start(&self, _name: &'static str) {
                self.start.fetch_add(1, Ordering::Relaxed);
            }
            fn on_module_built(&self, _name: &'static str, _elapsed: Duration) {
                self.built.fetch_add(1, Ordering::Relaxed);
            }
        }

        let obs: Box<dyn BuildObserver> = Box::new(Counting {
            start: Arc::clone(&start_count),
            built: Arc::clone(&built_count),
        });

        obs.on_module_start("m1");
        obs.on_module_built("m1", Duration::from_millis(1));
        assert_eq!(start_count.load(Ordering::Relaxed), 1);
        assert_eq!(built_count.load(Ordering::Relaxed), 1);
    }
}
