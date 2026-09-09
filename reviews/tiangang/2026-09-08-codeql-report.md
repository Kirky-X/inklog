# CodeQL Rust 深度扫描报告 — inklog

- 日期: 2026-09-09
- 目标: /home/kirky/projects/base/inklog (workspace, features: sqlite,http,cli)
- 工具: CodeQL CLI/bundle 2.27.0 (/home/kirky/tools/codeql)
- 数据库: /tmp/codeql-db (cargo build 成功, 0 编译错误)
- 套件: codeql/rust-queries:codeql-suites/rust-security-and-quality.qls (41 个查询全部执行)
- 覆盖: 163 个 Rust 文件成功提取, 16 个文件提取报错(Rust 支持较新, 主要为 proc-macro/生成代码)
- SARIF: /tmp/codeql-out/codeql-rust.sarif

## 发现统计(按 security-severity)

| 规则 | 数量 | 严重度(security-severity) | CWE |
|---|---|---|---|
| rust/hard-coded-cryptographic-value | 44 | 9.8 | CWE-259/321 |
| rust/cleartext-logging | 5 | 7.5 | CWE-312/359 |
| rust/log-injection | 6 | 6.1 | CWE-117 |
| **合计** | **55** | | |

质量类查询(rust/unused-variable 等)已运行, 0 项触发。

## 分级汇总
- security-severity 9.8(高危): 44 项 — 全部为硬编码密码/盐
- security-severity 7.5(中高): 5 项 — 明文写入日志
- security-severity 6.1(中): 6 项 — 日志注入

## 逐条发现
1. [sev=9.8] rust/hard-coded-cryptographic-value — src/cli/decrypt.rs:870 — This hard-coded value is used as [a password](1).
2. [sev=9.8] rust/hard-coded-cryptographic-value — src/cli/decrypt.rs:875 — This hard-coded value is used as [a password](1).
3. [sev=9.8] rust/hard-coded-cryptographic-value — src/cli/decrypt.rs:880 — This hard-coded value is used as [a password](1).
4. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:269 — This hard-coded value is used as [a password](1).
5. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:278 — This hard-coded value is used as [a password](1).
6. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:279 — This hard-coded value is used as [a password](1).
7. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:287 — This hard-coded value is used as [a password](1).
8. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:288 — This hard-coded value is used as [a password](1).
9. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:296 — This hard-coded value is used as [a password](1).
10. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:297 — This hard-coded value is used as [a password](1).
11. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:305 — This hard-coded value is used as [a password](1).
12. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:310 — This hard-coded value is used as [a password](1).
13. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:381 — This hard-coded value is used as [a password](1).
14. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:390 — This hard-coded value is used as [a password](1).
15. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:399 — This hard-coded value is used as [a password](1).
16. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:406 — This hard-coded value is used as [a password](1).
17. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:451 — This hard-coded value is used as [a password](1).
18. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:477 — This hard-coded value is used as [a password](1).
19. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:481 — This hard-coded value is used as [a password](1).
20. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:495 — This hard-coded value is used as [a salt](1).
21. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:501 — This hard-coded value is used as [a salt](1).
22. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:506 — This hard-coded value is used as [a password](1).
23. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:524 — This hard-coded value is used as [a salt](1).
24. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:540 — This hard-coded value is used as [a salt](1).
25. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/encryption.rs:553 — This hard-coded value is used as [a salt](1).
26. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/file.rs:1687 — This hard-coded value is used as [a salt](1).
27. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/file.rs:1875 — This hard-coded value is used as [a salt](1).
28. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/file.rs:1890 — This hard-coded value is used as [a salt](1).
29. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/file.rs:1985 — This hard-coded value is used as [a salt](1).
30. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/file.rs:3296 — This hard-coded value is used as [a salt](1).
31. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/file.rs:3299 — This hard-coded value is used as [a salt](1).
32. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/file.rs:3303 — This hard-coded value is used as [a salt](1).
33. [sev=9.8] rust/hard-coded-cryptographic-value — tests/e2e/e2e_advanced.rs:934 — This hard-coded value is used as [a password](1).
34. [sev=9.8] rust/hard-coded-cryptographic-value — tests/e2e/e2e_advanced.rs:936 — This hard-coded value is used as [a password](1).
35. [sev=9.8] rust/hard-coded-cryptographic-value — tests/e2e/e2e_advanced.rs:946 — This hard-coded value is used as [a password](1).
36. [sev=9.8] rust/hard-coded-cryptographic-value — tests/e2e/e2e_advanced.rs:948 — This hard-coded value is used as [a password](1).
37. [sev=9.8] rust/hard-coded-cryptographic-value — tests/e2e/e2e_advanced.rs:955 — This hard-coded value is used as [a password](1).
38. [sev=9.8] rust/hard-coded-cryptographic-value — tests/e2e/e2e_advanced.rs:957 — This hard-coded value is used as [a password](1).
39. [sev=9.8] rust/hard-coded-cryptographic-value — tests/e2e/e2e_advanced.rs:967 — This hard-coded value is used as [a password](1).
40. [sev=9.8] rust/hard-coded-cryptographic-value — tests/e2e/e2e_advanced.rs:975 — This hard-coded value is used as [a password](1).
41. [sev=9.8] rust/hard-coded-cryptographic-value — tests/e2e/e2e_advanced.rs:976 — This hard-coded value is used as [a password](1).
42. [sev=9.8] rust/hard-coded-cryptographic-value — tests/e2e/e2e_advanced.rs:983 — This hard-coded value is used as [a password](1).
43. [sev=9.8] rust/hard-coded-cryptographic-value — tests/e2e/e2e_advanced.rs:990 — This hard-coded value is used as [a password](1).
44. [sev=9.8] rust/hard-coded-cryptographic-value — src/support/io/sink/file.rs:1002 — This hard-coded value is used as [a salt](1).
45. [sev=7.5] rust/cleartext-logging — examples/tests/integration_test.rs:95 — This operation writes [derive_key_from_password(...)](1) to a log file.
46. [sev=7.5] rust/cleartext-logging — src/support/io/sink/file.rs:4398 — This operation writes [derive_key_from_password(...)](1) to a log file.
47. [sev=7.5] rust/cleartext-logging — examples/src/bin/security/encryption.rs:373 — This operation writes [derive_key_from_password(...)](1) to a log file.
48. [sev=7.5] rust/cleartext-logging — examples/src/bin/security/masking.rs:68 — This operation writes [bank_card](1) to a log file.
49. [sev=7.5] rust/cleartext-logging — examples/src/bin/security/masking.rs:69 — This operation writes [bank_card](1) to a log file.
50. [sev=6.1] rust/log-injection — src/domain/core/manager.rs:2002 — Log entry depends on a [user-provided value](1).
51. [sev=6.1] rust/log-injection — src/domain/core/manager.rs:2325 — Log entry depends on a [user-provided value](1).
52. [sev=6.1] rust/log-injection — src/domain/core/manager.rs:2385 — Log entry depends on a [user-provided value](1).
53. [sev=6.1] rust/log-injection — examples/src/bin/network/http.rs:453 — Log entry depends on a [user-provided value](1).
54. [sev=6.1] rust/log-injection — examples/src/bin/network/http.rs:501 — Log entry depends on a [user-provided value](1).
55. [sev=6.1] rust/log-injection — examples/src/bin/network/http.rs:504 — Log entry depends on a [user-provided value](1).