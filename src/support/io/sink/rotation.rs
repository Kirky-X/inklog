// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Log file rotation strategies.
//!
//! This module provides a strategy pattern implementation for log file rotation,
//! allowing flexible rotation policies based on size, time, or custom criteria.

use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Result of a rotation check
#[derive(Debug, Clone, Default)]
pub struct RotationResult {
    /// Whether rotation should occur
    pub should_rotate: bool,
    /// Reason for rotation (if applicable)
    pub reason: Option<String>,
    /// Suggested new file path (if applicable)
    pub new_path: Option<PathBuf>,
}

/// Context provided to rotation strategies
#[derive(Debug, Clone)]
pub struct RotationContext {
    /// Current file path
    pub current_path: PathBuf,
    /// Current file size in bytes
    pub current_size: u64,
    /// Maximum allowed size (if configured)
    pub max_size: Option<u64>,
    /// Time when current file was opened
    pub file_opened_at: Instant,
    /// Last rotation time
    pub last_rotation: Instant,
    /// Current timestamp
    pub now: DateTime<Utc>,
    /// Current sequence number
    pub sequence: u32,
}

/// Trait for log file rotation strategies.
///
/// Implement this trait to define custom rotation policies.
pub trait RotationStrategy: Send + Sync {
    /// Check if rotation should occur based on the given context.
    fn should_rotate(&self, context: &RotationContext) -> RotationResult;

    /// Get the name of this strategy for logging/metrics.
    fn name(&self) -> &'static str;

    /// Generate the next file path after rotation.
    fn generate_next_path(&self, base_path: &Path, context: &RotationContext) -> PathBuf;

    /// Clone the strategy into a boxed trait object.
    fn clone_boxed(&self) -> Box<dyn RotationStrategy>;
}

/// Size-based rotation strategy.
///
/// Rotates the log file when it exceeds a configured maximum size.
#[derive(Debug, Clone)]
pub struct SizeBasedRotation {
    /// Maximum file size in bytes before rotation
    max_size: u64,
}

impl SizeBasedRotation {
    /// Create a new size-based rotation strategy.
    pub fn new(max_size: u64) -> Self {
        Self { max_size }
    }

    /// Create from a human-readable size string (e.g., "100MB", "1GB").
    pub fn from_size_string(size_str: &str) -> Result<Self, String> {
        let max_size = parse_size(size_str)?;
        Ok(Self { max_size })
    }

    /// Get the maximum size threshold.
    pub fn max_size(&self) -> u64 {
        self.max_size
    }
}

impl RotationStrategy for SizeBasedRotation {
    fn should_rotate(&self, context: &RotationContext) -> RotationResult {
        if context.current_size >= self.max_size {
            RotationResult {
                should_rotate: true,
                reason: Some(format!(
                    "File size {} bytes exceeds limit {} bytes",
                    context.current_size, self.max_size
                )),
                new_path: Some(self.generate_next_path(&context.current_path, context)),
            }
        } else {
            RotationResult::default()
        }
    }

    fn name(&self) -> &'static str {
        "size_based"
    }

    fn generate_next_path(&self, base_path: &Path, context: &RotationContext) -> PathBuf {
        let timestamp = context.now.format("%Y%m%d_%H%M%S");
        let seq = context.sequence;

        let stem = base_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("log");
        let ext = base_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("log");

        let parent = base_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));

        parent.join(format!("{}_{}_{}.{}", stem, timestamp, seq, ext))
    }

    fn clone_boxed(&self) -> Box<dyn RotationStrategy> {
        Box::new(self.clone())
    }
}

/// Time-based rotation strategy.
///
/// Rotates the log file based on time intervals (hourly, daily, weekly, monthly).
#[derive(Debug, Clone)]
pub struct TimeBasedRotation {
    /// Rotation interval in seconds
    interval_secs: u64,
    /// Interval name for logging
    interval_name: String,
}

impl TimeBasedRotation {
    /// Create a new time-based rotation strategy.
    pub fn new(interval_secs: u64, interval_name: String) -> Self {
        Self {
            interval_secs,
            interval_name,
        }
    }

    /// Create from a string identifier (hourly, daily, weekly, monthly).
    pub fn from_interval_string(interval: &str) -> Result<Self, String> {
        let (secs, name) = match interval.to_lowercase().as_str() {
            "hourly" => (3600, "hourly".to_string()),
            "daily" => (86400, "daily".to_string()),
            "weekly" => (604800, "weekly".to_string()),
            "monthly" => (2592000, "monthly".to_string()),
            _ => return Err(format!("Unknown rotation interval: {}", interval)),
        };
        Ok(Self {
            interval_secs: secs,
            interval_name: name,
        })
    }

    /// Get the rotation interval in seconds.
    pub fn interval_secs(&self) -> u64 {
        self.interval_secs
    }

    /// Check if the date has changed (for daily/monthly rotation).
    fn has_date_changed(&self, context: &RotationContext) -> bool {
        let elapsed = context.last_rotation.elapsed().as_secs();
        elapsed >= self.interval_secs
    }
}

impl RotationStrategy for TimeBasedRotation {
    fn should_rotate(&self, context: &RotationContext) -> RotationResult {
        if self.has_date_changed(context) {
            RotationResult {
                should_rotate: true,
                reason: Some(format!("Time interval {} elapsed", self.interval_name)),
                new_path: Some(self.generate_next_path(&context.current_path, context)),
            }
        } else {
            RotationResult::default()
        }
    }

    fn name(&self) -> &'static str {
        "time_based"
    }

    fn generate_next_path(&self, base_path: &Path, context: &RotationContext) -> PathBuf {
        let timestamp = context.now.format("%Y%m%d_%H%M%S");
        let seq = context.sequence;

        let stem = base_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("log");
        let ext = base_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("log");

        let parent = base_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));

        parent.join(format!("{}_{}_{}.{}", stem, timestamp, seq, ext))
    }

    fn clone_boxed(&self) -> Box<dyn RotationStrategy> {
        Box::new(self.clone())
    }
}

/// Combined rotation strategy that checks multiple conditions.
///
/// Rotation occurs when ANY of the configured strategies triggers.
pub struct CompositeRotation {
    strategies: Vec<Box<dyn RotationStrategy>>,
}

impl std::fmt::Debug for CompositeRotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeRotation")
            .field("strategies_count", &self.strategies.len())
            .finish()
    }
}

impl CompositeRotation {
    /// Create a new composite rotation strategy.
    pub fn new(strategies: Vec<Box<dyn RotationStrategy>>) -> Self {
        Self { strategies }
    }

    /// Add a strategy to the composite.
    pub fn add<S: RotationStrategy + 'static>(&mut self, strategy: S) {
        self.strategies.push(Box::new(strategy));
    }
}

impl RotationStrategy for CompositeRotation {
    fn should_rotate(&self, context: &RotationContext) -> RotationResult {
        for strategy in &self.strategies {
            let result = strategy.should_rotate(context);
            if result.should_rotate {
                return result;
            }
        }
        RotationResult::default()
    }

    fn name(&self) -> &'static str {
        "composite"
    }

    fn generate_next_path(&self, base_path: &Path, context: &RotationContext) -> PathBuf {
        if let Some(strategy) = self.strategies.first() {
            strategy.generate_next_path(base_path, context)
        } else {
            base_path.to_path_buf()
        }
    }

    fn clone_boxed(&self) -> Box<dyn RotationStrategy> {
        let cloned_strategies: Vec<Box<dyn RotationStrategy>> =
            self.strategies.iter().map(|s| s.clone_boxed()).collect();
        Box::new(Self::new(cloned_strategies))
    }
}

/// Parse a size string into bytes.
///
/// Supports formats like "100MB", "1GB", "500KB", etc.
pub fn parse_size(size_str: &str) -> Result<u64, String> {
    let size_str = size_str.trim().to_uppercase();

    let (multiplier, suffix_len) = if size_str.ends_with("GB") {
        (1024 * 1024 * 1024, 2)
    } else if size_str.ends_with("MB") {
        (1024 * 1024, 2)
    } else if size_str.ends_with("KB") {
        (1024, 2)
    } else if size_str.ends_with("B") {
        (1, 1)
    } else {
        (1, 0)
    };

    let num_str = &size_str[..size_str.len() - suffix_len];
    let num: u64 = num_str
        .parse()
        .map_err(|_| format!("Invalid size number: {}", num_str))?;

    Ok(num * multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_context(size: u64, elapsed_secs: u64) -> RotationContext {
        RotationContext {
            current_path: PathBuf::from("/var/log/app.log"),
            current_size: size,
            max_size: None,
            file_opened_at: Instant::now() - Duration::from_secs(elapsed_secs),
            last_rotation: Instant::now() - Duration::from_secs(elapsed_secs),
            now: Utc::now(),
            sequence: 1,
        }
    }

    #[test]
    fn test_size_based_rotation() {
        let strategy = SizeBasedRotation::new(1000);

        let context = create_context(500, 0);
        let result = strategy.should_rotate(&context);
        assert!(!result.should_rotate);

        let context = create_context(1500, 0);
        let result = strategy.should_rotate(&context);
        assert!(result.should_rotate);
        assert!(result.reason.unwrap().contains("exceeds limit"));
    }

    #[test]
    fn test_size_based_rotation_from_str() {
        let strategy = SizeBasedRotation::from_size_string("100MB").unwrap();
        assert_eq!(strategy.max_size(), 100 * 1024 * 1024);

        let strategy = SizeBasedRotation::from_size_string("1GB").unwrap();
        assert_eq!(strategy.max_size(), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_time_based_rotation() {
        let strategy = TimeBasedRotation::from_interval_string("daily").unwrap();

        let context = create_context(100, 3600);
        let result = strategy.should_rotate(&context);
        assert!(!result.should_rotate);

        let context = create_context(100, 86400);
        let result = strategy.should_rotate(&context);
        assert!(result.should_rotate);
    }

    #[test]
    fn test_composite_rotation() {
        let mut composite = CompositeRotation::new(vec![]);
        composite.add(SizeBasedRotation::new(1000));
        composite.add(TimeBasedRotation::from_interval_string("daily").unwrap());

        let context = create_context(500, 3600);
        let result = composite.should_rotate(&context);
        assert!(!result.should_rotate);

        let context = create_context(1500, 3600);
        let result = composite.should_rotate(&context);
        assert!(result.should_rotate);

        let context = create_context(500, 86400);
        let result = composite.should_rotate(&context);
        assert!(result.should_rotate);
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("100B").unwrap(), 100);
        assert_eq!(parse_size("1KB").unwrap(), 1024);
        assert_eq!(parse_size("1MB").unwrap(), 1024 * 1024);
        assert_eq!(parse_size("1GB").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size("100mb").unwrap(), 100 * 1024 * 1024);
    }

    #[test]
    fn test_generate_next_path() {
        let strategy = SizeBasedRotation::new(1000);
        let context = create_context(100, 0);

        let path = strategy.generate_next_path(&PathBuf::from("/var/log/app.log"), &context);
        assert!(path.to_str().unwrap().contains("app_"));
        assert!(path.to_str().unwrap().ends_with(".log"));
    }
}
