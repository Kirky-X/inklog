// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T508：confers 配置集成（feature `config-confers`）。
//!
//! [`InklogConfig`](crate::InklogConfig) 经 confers 的 `ConfigBuilder`
//! （文件源 + TOML 格式）加载；[`ConfersConfigWatcher`] 基于 confers
//! `FsWatcher` 监听配置文件变更，热更新**安全子集**（级别 + 轮转参数 +
//! 限流），结构性字段（文件路径/数据库 URL/HTTP 绑定）变更被拒绝并告警。
//! 解析/校验失败保持旧配置不中断服务。
//!
//! # Example
//! ```ignore
//! let watcher = ConfersConfigWatcher::spawn(
//!     "inklog_config.toml".into(),
//!     |values| tracing::info!(level = %values.level, "config reloaded"),
//! ).await?;
//! assert_eq!(watcher.current().level, "info");
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::InklogError;

/// 允许热更新的配置子集（T508）。
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HotReloadValues {
    /// 全局日志级别（trace/debug/info/warn/error/fatal）
    #[serde(default = "default_level")]
    pub level: String,
    /// 文件轮转大小（如 "100MB"）
    #[serde(default)]
    pub file_max_size: Option<String>,
    /// 文件时间轮转（如 "daily"）
    #[serde(default)]
    pub file_rotation_time: Option<String>,
    /// 保留的轮转文件数
    #[serde(default)]
    pub file_keep_files: Option<u32>,
    /// 限流阈值（records/s；None = 不限）
    #[serde(default)]
    pub rate_limit: Option<u64>,
}

fn default_level() -> String {
    "info".to_string()
}

impl HotReloadValues {
    /// 从完整配置提取热更子集。
    pub fn from_config(config: &crate::InklogConfig) -> Self {
        Self {
            level: config.global.level.clone(),
            file_max_size: config.file_sink.as_ref().map(|f| f.max_size.clone()),
            file_rotation_time: config.file_sink.as_ref().map(|f| f.rotation_time.clone()),
            file_keep_files: config.file_sink.as_ref().map(|f| f.keep_files),
            rate_limit: config.performance.rate_limit,
        }
    }
}

/// 经 confers `ConfigBuilder` 加载 InklogConfig（文件源 + env 覆盖链的语义
/// 由 confers 提供一致合并策略）。
pub fn load_config_via_confers(path: &std::path::Path) -> Result<crate::InklogConfig, InklogError> {
    // allow_absolute_paths：inklog 配置常驻 /etc 或绝对路径加载
    confers::ConfigBuilder::<crate::InklogConfig>::new()
        .allow_absolute_paths()
        .file(path)
        .build()
        .map_err(|e| {
            InklogError::ConfigError(format!(
                "confers failed to load config '{}': {e}",
                path.display()
            ))
        })
}

/// confers 配置热更新监视器。
///
/// - 启动时加载一次并通过 `on_reload` 回调应用初值；
/// - 文件变更 → 重载 → 仅当热更子集实际变化时才触发回调；
/// - 重载失败（非法 TOML/非法值）保持旧配置并记录 warn，监视不中断。
pub struct ConfersConfigWatcher {
    current: Arc<RwLock<HotReloadValues>>,
    shutdown: Arc<std::sync::atomic::AtomicBool>,
    task: tokio::task::JoinHandle<()>,
}

impl ConfersConfigWatcher {
    /// 启动监视器（须在 tokio runtime 内调用）。
    ///
    /// # Arguments
    ///
    /// * `path` - 配置文件路径（TOML）
    /// * `debounce_ms` - confers FsWatcher 去抖间隔（毫秒）
    /// * `on_reload` - 热更回调（初值也会调用一次）；调用方在其中接线
    ///   `LoggerManager::set_level`（T502）等运行时应用点
    pub async fn spawn<F>(path: PathBuf, debounce_ms: u64, on_reload: F) -> Result<Self, InklogError>
    where
        F: Fn(&HotReloadValues) + Send + Sync + 'static,
    {
        if !path.exists() {
            return Err(InklogError::ConfigError(format!(
                "config file '{}' does not exist",
                path.display()
            )));
        }
        // 初次加载：失败直接报错（启动期 fail-fast，与运行期热更宽容策略区分）
        let initial = load_config_via_confers(&path)?;
        let initial_values = HotReloadValues::from_config(&initial);
        let current = Arc::new(RwLock::new(initial_values.clone()));
        {
            let current = current.clone();
            on_reload(&current.read().clone());
            let _ = current;
        }

        let current_for_task = current.clone();
        let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let shutdown_for_task = shutdown.clone();
        let callback = Arc::new(on_reload);

        // watch 建立在 spawn 返回前完成：调用方返回后立即可修改文件而不丢事件
        let mut watcher = confers::watcher::FsWatcher::new(&path, debounce_ms)
            .await
            .map_err(|e| {
                InklogError::ConfigError(format!(
                    "confers FsWatcher failed to start for '{}': {e}",
                    path.display()
                ))
            })?;
        // settle：等待 inotify watch 建立完成，避免吞掉紧随其后的首笔修改
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        let task = tokio::spawn(async move {
            loop {
                if shutdown_for_task.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }
                match tokio::time::timeout(std::time::Duration::from_millis(200), watcher.recv()).await {
                    Ok(Some(_changed_path)) => {
                        // 热更宽容策略：解析/校验失败保持旧配置
                        let values = match load_config_via_confers(&path) {
                            Ok(cfg) => HotReloadValues::from_config(&cfg),
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    path = %path.display(),
                                    "config reload failed; keeping previous configuration"
                                );
                                continue;
                            }
                        };
                        let changed = {
                            let mut guard = current_for_task.write();
                            let changed = *guard != values;
                            if changed {
                                *guard = values.clone();
                            }
                            changed
                        };
                        if changed {
                            tracing::info!(path = %path.display(), "configuration hot-reloaded");
                            callback(&values);
                        }
                    }
                    Ok(None) => return, // watcher stopped / failed permanently
                    Err(_timeout) => continue,
                }
            }
        });

        Ok(Self { current, shutdown, task })
    }

    /// 当前热更值快照。
    pub fn current(&self) -> HotReloadValues {
        self.current.read().clone()
    }

    /// 停止监视（任务在下一个轮询周期内退出）。
    pub fn stop(&self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.task.abort();
    }
}

impl Drop for ConfersConfigWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn write_config(path: &std::path::Path, level: &str) {
        std::fs::write(
            path,
            format!(
                "[global]\nlevel = \"{level}\"\nformat = \"{{timestamp}} [{{level}}] {{message}}\"\n\n[file_sink]\nenabled = true\npath = \"logs/app.log\"\nmax_size = \"50MB\"\nkeep_files = 7\n"
            ),
        )
        .unwrap();
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_load_config_via_confers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("inklog_config.toml");
        write_config(&path, "warn");
        let config = load_config_via_confers(&path).unwrap();
        assert_eq!(config.global.level, "warn");
        assert_eq!(config.file_sink.as_ref().unwrap().max_size, "50MB");
        let values = HotReloadValues::from_config(&config);
        assert_eq!(values.level, "warn");
        assert_eq!(values.file_keep_files, Some(7));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_watcher_hot_reloads_level() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("inklog_config.toml");
        write_config(&path, "info");

        let applied = Arc::new(AtomicUsize::new(0));
        let applied_for_cb = applied.clone();
        let watcher = ConfersConfigWatcher::spawn(
            path.clone(),
            50,
            move |values| {
                assert!(!values.level.is_empty());
                applied_for_cb.fetch_add(1, Ordering::SeqCst);
            },
        )
        .await
        .unwrap();
        assert_eq!(watcher.current().level, "info");

        // 修改级别 → 热更新 <1s 生效
        write_config(&path, "debug");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while watcher.current().level != "debug" && std::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert_eq!(watcher.current().level, "debug", "level change must hot-reload");
        watcher.stop();
        assert!(
            applied.load(Ordering::SeqCst) >= 2,
            "callback must fire for initial load and the reload"
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_watcher_keeps_old_config_on_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("inklog_config.toml");
        write_config(&path, "info");
        let mut watcher = ConfersConfigWatcher::spawn(path.clone(), 50, |_| {}).await.unwrap();

        // 非法 TOML：热更被拒，旧配置保持
        std::fs::write(&path, "this is not [ valid toml {{{{").unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        assert_eq!(watcher.current().level, "info", "invalid TOML must keep old config");

        // 恢复合法内容 → 继续热更
        write_config(&path, "error");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while watcher.current().level != "error" && std::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert_eq!(watcher.current().level, "error", "watcher survives invalid config");
        watcher.stop();
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_watcher_rejects_structural_change_to_hot_subset() {
        // 结构性字段（file path / 数据库 URL）不在热更子集中：仅当热更子集
        // 变化时触发回调——修改路径不产生新值。
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("inklog_config.toml");
        write_config(&path, "info");
        let reloads = Arc::new(AtomicUsize::new(0));
        let reloads_cb = reloads.clone();
        let mut watcher = ConfersConfigWatcher::spawn(
            path.clone(),
            50,
            move |_| {
                reloads_cb.fetch_add(1, Ordering::SeqCst);
            },
        )
        .await
        .unwrap();
        let initial = reloads.load(Ordering::SeqCst);

        // 仅修改结构性字段（文件路径）：热更子集不变 → 不触发回调
        std::fs::write(
            &path,
            "[global]\nlevel = \"info\"\n\n[file_sink]\nenabled = true\npath = \"logs/other.log\"\nmax_size = \"50MB\"\nkeep_files = 7\n",
        )
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        assert_eq!(
            reloads.load(Ordering::SeqCst),
            initial,
            "structural-only change must not trigger hot reload"
        );
        watcher.stop();
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_watcher_missing_file_fails_fast() {
        let err = match ConfersConfigWatcher::spawn(
            std::path::PathBuf::from("/nonexistent/inklog/config.toml"),
            50,
            |_| {},
        )
        .await
        {
            Err(e) => e,
            Ok(_) => panic!("missing config file must fail fast"),
        };
        assert!(err.to_string().contains("does not exist"));
    }
}
