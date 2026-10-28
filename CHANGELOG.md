# Changelog

所有显著变更记录于此。
格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

## Unreleased

### Changed

- 掩码引擎下沉 trait-kit（`masking` feature，单一来源）：本仓删除
  `masking` / `masking_ac` / `masking_registry` / `secret_scan` / `secret_patterns` /
  `secret_entropy` 六文件，`inklog::DataMasker` / `MaskRule` / `MaskRuleRegistry` /
  `SecretScanGate` 等公开路径经 re-export 保持兼容；`fast-masking` 保留为 no-op
  兼容别名（Aho-Corasick 加速随 trait-kit/masking 常载）。掩码 API 错误类型随
  引擎变为 trait-kit 的 `MaskError`（本仓 `InklogError` 的 `SecretScan*` 变体
  保留但不再由掩码路径产生）。

### Added

- `trait-kit` 转为必选依赖（`features = ["masking"]`）；`kit` feature 改以
  `trait-kit/<f>` 引用（依赖已非可选）。
