# Spec — pbkdf2-guard

> Delta spec for change `fix-review-findings`. 覆盖此变更引入/修改的 PBKDF2 迭代数守卫需求。

## Requirements

### R-pbkdf2-guard-001: 迭代数具名常量单一事实来源

`src/support/io/sink/encryption.rs` 顶部定义 `pub(crate) const PBKDF2_ITERATIONS: u32 = 600_000;`，`derive_key_from_password` 的 `pbkdf2_hmac` 调用必须引用该常量，不得存在其他内联迭代数字面量。

**验收标准：**
- `grep -n "600_000\|600000" src/support/io/sink/encryption.rs` 仅命中常量定义一处（及引用该常量的测试）
- `grep -n "PBKDF2_ITERATIONS" src/support/io/sink/encryption.rs` 显示 `pbkdf2_hmac` 调用点使用常量

### R-pbkdf2-guard-002: 测试真实守卫生产常量

`test_pbkdf2_iteration_count_is_at_least_600k` 必须同时满足：(a) 断言 `PBKDF2_ITERATIONS >= 600_000`；(b) 经生产路径 `derive_key_from_password(password, Some(fixed_salt))` 得到的密钥与本地 `pbkdf2_hmac(password, salt, PBKDF2_ITERATIONS)` 结果逐字节一致。

**验收标准：**
- 测试通过 `cargo test -p inklog --lib encryption`
- 反向验证：将 `PBKDF2_ITERATIONS` 临时改为 `1_000` 后 (a) 断言失败；将派生路径改回内联字面量 `1_000` 后 (b) 比对失败（验证后还原）

### R-pbkdf2-guard-003: 32 字节兼容语义 rustdoc 显式化

`get_encryption_key` 的 rustdoc 包含显式的兼容性小节，说明"长度恰为 32 字节的输入被直接用作原始密钥（不经过 PBKDF2），为解密既有加密文件的兼容行为，新部署应使用 Base64 编码随机密钥或长度非 32 的密码"。

**验收标准：**
- `cargo doc` 无警告，且 `grep -A 3 "兼容性" src/support/io/sink/encryption.rs` 命中该小节

## Constraints

- 常量可见性 `pub(crate)`，不扩大公共 API
- 派生结果必须与既有加密文件兼容：对同一 (password, salt) 派生输出与修复前逐字节一致（迭代数未变）

## Out of Scope

- 迭代数可配置化；密码策略变更；LOW-003 参数化 SQL 迁移
