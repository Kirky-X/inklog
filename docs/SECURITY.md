# 🔒 inklog 安全文档

本文档描述 inklog 的安全设计（加密、数据脱敏、内存安全、访问控制、网络安全与合规性）、漏洞报告流程，以及使用 inklog 处理敏感日志时的安全最佳实践。

> 相关文档：[📖 用户指南](USER_GUIDE.md) · [🏗️ 架构设计](ARCHITECTURE.md) · [🤝 贡献指南](CONTRIBUTING.md)

<details open>
<summary>📑 目录</summary>

- [📌 支持版本](#-支持版本)
- [🚨 漏洞报告流程](#-漏洞报告流程)
- [🛡️ 安全设计概览](#️-安全设计概览)
- [🔐 加密与密钥管理](#-加密与密钥管理)
- [🎭 数据脱敏](#-数据脱敏)
- [🧠 内存安全](#-内存安全)
- [🚪 访问控制](#-访问控制)
- [🌐 网络安全](#-网络安全)
- [📜 合规性](#-合规性)
- [✅ 安全最佳实践](#-安全最佳实践)
- [📎 附录](#-附录)

</details>

---

## 📌 支持版本

inklog 目前处于 0.x 预发布阶段（当前版本 **0.3.0-rc.3**），暂无 1.0 长期支持（LTS）分支。安全修复随最新发布版本发布，建议始终升级到最新版本。

| 版本 | 发布日期 | 说明 |
|------|----------|------|
| 0.3.0-rc.3 | 2026-09-10 | 最新版本 |
| 0.3.0-rc.2 | 2026-09-03 | 历史版本 |
| 0.2.0 | 2026-08-05 | 历史版本 |

> 历史安全修复示例：0.1.11 修复了 `table_name` SQL 注入防护、`FileSink` 路径穿越防护与 HTTP 认证 fail-closed 三项安全问题（详见 [更新日志](CHANGELOG.md)）。

## 🚨 漏洞报告流程

如果您发现 inklog 的安全漏洞，请负责任地向我们报告。

### 报告原则

**❌ 避免**：

- 在 GitHub Issues 中公开安全漏洞；
- 在社交媒体上披露漏洞细节；
- 在未经授权的情况下进行漏洞利用。

**✅ 推荐**：

- 通过私密渠道报告漏洞；
- 提供复现步骤和影响评估；
- 给予足够的时间修复。

### 报告方式

**首选方式**：

- 邮件：[Kirky-X@outlook.com](mailto:Kirky-X@outlook.com)
- PGP 密钥：(将在安全页面提供)

**备选方式**：

- [GitHub Security Advisories](https://github.com/Kirky-X/inklog/security/advisories)

### 报告内容

| 部分 | 应包含的信息 |
|------|--------------|
| **漏洞描述** | 漏洞类型（注入、认证绕过等）、影响范围（受影响版本）、严重程度（CVSS 评分） |
| **复现步骤** | 详细的重现步骤、代码示例或配置、预期行为与实际行为 |
| **影响评估** | 数据泄露可能性、系统可用性影响、业务影响范围 |
| **缓解措施** | 临时缓解方案、建议的修复方向 |

### 响应时间表

| 阶段 | 时间 | 行动 |
|------|------|------|
| **确认收到** | 24 小时内 | 确认收到安全报告 |
| **初步评估** | 48 小时内 | 评估漏洞严重性 |
| **修复开发** | 7-14 天 | 开发并测试修复 |
| **补丁发布** | 修复完成后 | 发布安全补丁 |
| **公开披露** | 修复后 7-30 天 | 发布安全公告（协调披露） |

### 协调披露流程

```text
[Day 0]  研究者报告漏洞
         ↓
[Day 1]   inklog 确认并评估
         ↓
[Day 7]   修复开发完成
         ↓
[Day 10]  补丁发布到私有预览
         ↓
[Day 14]  公开发布 + 安全公告
```

**影响因素**：漏洞严重程度（严重漏洞优先处理）、修复复杂度、已知的公开利用情况。

### 安全赏金计划

#### 奖励范围

| 严重性 | CVSS 评分 | 奖励金额 | 示例 |
|--------|-----------|----------|------|
| **严重** | 9.0 - 10.0 | $1000 | RCE、SQL 注入、认证绕过 |
| **高危** | 7.0 - 8.9 | $500 | 敏感数据泄露、XSS |
| **中等** | 4.0 - 6.9 | $250 | CSRF、信息泄露 |
| **低** | 0.1 - 3.9 | $100 | 轻微安全问题 |

#### 排除范围

以下问题不符合奖励条件：

- 已知漏洞的重复报告；
- 需要物理访问的漏洞；
- 社会工程攻击；
- 第三方依赖的漏洞；
- 最佳实践违规（非安全漏洞）。

### 已知安全问题与订阅

查看当前已知安全问题：

- **GitHub Security Advisories**：<https://github.com/Kirky-X/inklog/security/advisories>
- **更新日志**：[CHANGELOG](CHANGELOG.md)

订阅安全更新：在 GitHub 仓库点击 **Watch → Custom**，勾选 **Releases** 与 **Security alerts**。

### 安全资源

**学习资源**：

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [CWE Top 25](https://cwe.mitre.org/top25/)
- [RustSec Advisory DB](https://github.com/rustsec/advisory-db)

**工具推荐**：

- `cargo-audit`：依赖漏洞扫描
- `cargo-deny`：许可证和来源检查

## 🛡️ 安全设计概览

inklog 是安全优先的 Rust 日志基础设施，为需要严格数据保护和合规性要求的环境设计。

### 设计原则

- **加密优先**：AES-256-GCM 认证加密，支持密钥管理服务集成；
- **零信任**：所有敏感数据通过环境变量或 KMS 注入，从不硬编码；
- **内存安全**：使用 `zeroize` 确保敏感数据在内存中不留痕迹；
- **深度防御**：多层安全控制，从加密、脱敏到访问控制；
- **合规性设计**：支持 GDPR、HIPAA、PCI-DSS 等合规要求。

### 安全能力总览

| 能力 | 实现位置 | 说明 |
|------|----------|------|
| 静态加密 | `support::io::sink::encryption` | AES-256-GCM，仅作用于轮转归档 |
| 密钥派生 | 同上 | PBKDF2-HMAC-SHA256 600k 迭代，每次轮转至多一次 |
| 密钥内存清零 | `zeroize` | 密钥离开作用域自动清零 |
| KMS 密钥提供 | `support::security`（`kms` feature） | `EnvKeyProvider` / `ConfersKeyProvider` / Vault transit MVP |
| 数据脱敏 | `support::processing::masking` | 21 条内置正则规则 + 敏感字段名检测 + 自定义注册表 |
| 路径安全 | `validation::path`（PathValidator） | 路径穿越防护，禁止写入用户主目录与密钥文件 |
| 内容净化 | `validation::sanitize`（LogSanitizer） | 日志注入与控制字符防护 |
| SQL 注入防护 | DatabaseSink / `integrations::infra::database` | 表名白名单校验 + 参数化查询 |
| HTTP 访问控制 | `http` feature | 认证 token 启动期缓存、失败 fail-closed；IP 白名单；TLS |
| 归档防篡改 | `support::audit_chain`（ArchiveChain） | HMAC-SHA256 归档链（随机链首盐），防删除、重排与伪造 |
| 供应链安全 | `deny.toml` / lefthook | `cargo deny check` + `cargo audit` + pre-commit 私钥扫描 |

> 说明：仓库维护 [`deny.toml`](../deny.toml)，CI 的 security job 与 lefthook pre-push 分别运行 `cargo deny check`（漏洞/许可证/重复依赖）与 `cargo audit`（RustSec 公告）。

## 🔐 加密与密钥管理

inklog 使用 **AES-256-GCM**（Galois/Counter Mode）进行认证加密，同时提供机密性与完整性保证。

### 加密流程

1. **密钥获取**：从环境变量安全读取 32 字节密钥（`Zeroizing` 包裹）；
2. **Nonce 生成**：加密安全的随机数生成器创建 12 字节 nonce；
3. **加密操作**：AES-256-GCM 加密数据；
4. **完整性验证**：GCM 模式自动包含认证标签；
5. **文件写入**：按版本化格式写入加密数据。

> 加密仅作用于**轮转归档**：活跃日志文件保持明文，轮转触发时先压缩（`.zst`）后后台加密（`.enc`），加密与密钥派生不占用单条记录的写入热路径。

### 密钥获取

```rust
// src/support/io/sink/encryption.rs
pub fn get_encryption_key(env_var: &str) -> Result<Zeroizing<[u8; 32]>, InklogError>
```

- `Zeroizing` 包裹的 32 字节密钥离开作用域时自动清零；
- 环境变量值为 Base64 编码的 32 字节时直接作密钥；为普通密码（1-127 字符）时经 PBKDF2-HMAC-SHA256（600,000 次迭代）派生；
- 低熵或长度不符的密钥会被拒绝（启动期报错）。

**密钥格式支持**：

| 格式 | 示例 | 说明 |
|------|------|------|
| **Base64** | `MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI=` | 推荐，易于管理 |
| **原始字节** | 32 字节二进制数据 | 需精确处理 |
| **密码字符串** | 1-127 字符 | 经 PBKDF2 派生（v2 格式支持） |

**环境变量配置**：

```bash
# 生成并设置加密密钥（推荐）
export INKLOG_ENCRYPTION_KEY=$(openssl rand -base64 32)
```

### 加密文件格式

#### v2 格式（当前版本）

```text
偏移      大小     描述
--------  -------  ------------------
0-7       8 字节   MAGIC: "ENCLOG1\0"
8-9       2 字节   版本号: 2 (u16 little-endian)
10-11     2 字节   算法标识: 1 (AES-256-GCM)
12-27     16 字节  PBKDF2 盐
28-39     12 字节  Nonce
40+       N 字节   密文（含 GCM 认证标签）
```

- 密钥来源两种：环境变量为 Base64/原始 32 字节时直接作密钥（盐存而不用，为格式统一）；为普通密码时经 PBKDF2-HMAC-SHA256（600,000 次迭代，OWASP 推荐）以头中盐做确定性派生，解密方从文件头读盐即可恢复；
- PBKDF2 600,000 次迭代是安全审查认可的最低迭代数，禁止为性能调低（单次派生约 63 ms，见 [⚡ 性能基线](PERFORMANCE.md)，每次轮转文件至多一次）。

#### v1 与 Legacy 格式（向后兼容）

| 格式 | 头部结构 | 说明 |
|------|----------|------|
| v1 | `[MAGIC 8][version 2][algo 2][nonce 12][密文]` | 无盐，仍可解密；v1 时代的密码模式因未存盐无法恢复（解密时显式报错），请迁移到 v2 |
| Legacy | `[MAGIC 8][version 2][nonce 12][密文]` | 早期格式，解密工具自动识别 |

### 密钥轮换

密钥轮换是加密安全的关键实践。

**为什么需要密钥轮换**：

- **降低密钥泄露风险**：定期更换密钥可限制泄露的影响范围；
- **合规要求**：PCI-DSS、HIPAA 等标准要求定期轮换密钥；
- **加密算法演进**：支持未来升级到更强的加密算法。

**轮换步骤**：

1. **生成新密钥**并配置到新的环境变量：

```bash
NEW_KEY=$(openssl rand -base64 32)
export INKLOG_ENCRYPTION_KEY_V2=$NEW_KEY
```

2. **切换配置**指向新密钥环境变量（`encryption_key_env = "INKLOG_ENCRYPTION_KEY_V2"`），新轮转归档即使用新密钥；
3. **迁移旧归档**：用 CLI 解密旧文件（支持 v1/v2 与 zstd/gzip 解包），解密产物按需由应用以新密钥重新落盘或导入数据库：

```bash
# 使用旧密钥解密（支持 --batch 批量与 --recursive 目录递归）
inklog-cli decrypt \
  --input logs/ \
  --output temp_decrypted/ \
  --batch \
  --recursive \
  --key-env INKLOG_OLD_ENCRYPTION_KEY
```

4. **更新密钥管理系统**（如 Kubernetes Secrets），清理旧密钥引用；
5. **验证**：对归档抽样执行解密验证。

**多密钥解密**：`inklog-cli query` 检索时自动尝试解密 `.enc` 文件，配合新旧密钥并存可实现平滑迁移。

**轮换最佳实践**：

| 实践 | 说明 |
|------|------|
| **轮换周期** | 建议 90 天轮换一次，高安全环境可缩短至 30 天 |
| **密钥存储** | 使用 KMS 或 Secrets Manager，不要硬编码（`kms` feature 提供 Vault transit MVP） |
| **备份策略** | 轮换前备份旧密钥和加密文件 |
| **审计日志** | 记录所有密钥轮换操作 |
| **测试验证** | 轮换后验证解密功能正常 |

**轮换检查清单**：

- [ ] 生成新的强密钥（32 字节，高熵值）
- [ ] 备份当前加密文件
- [ ] 配置新密钥到环境变量
- [ ] 验证新密钥加密功能
- [ ] 迁移旧加密文件
- [ ] 更新密钥管理系统
- [ ] 清理旧密钥引用
- [ ] 记录轮换审计日志
- [ ] 验证所有日志可正常解密

### 解密（CLI 工具）

```bash
# 解密单个文件
inklog-cli decrypt \
  --input logs/app_20260913_143022.log.zst.enc \
  --output logs/app_decrypted.log \
  --key-env INKLOG_DECRYPT_KEY

# 批量解密目录
inklog-cli decrypt \
  --input logs/ \
  --output decrypted/ \
  --batch \
  --recursive \
  --key-env INKLOG_DECRYPT_KEY
```

**解密验证**：

- 自动检测文件格式（v2 / v1 / Legacy）；
- 验证 MAGIC 头（`ENCLOG1\0`）；
- 检查算法标识（仅支持 AES-256-GCM）；
- GCM 认证标签自动验证数据完整性。

## 🎭 数据脱敏

inklog 内置全面的 **PII（个人身份信息）** 检测和脱敏功能，保护敏感数据不被记录到日志中。

### 内置规则（21 条，按优先级排序执行）

规则分为三个优先级组：高（10-25）、中（30-40）、低（50-100）。

#### 高优先级规则（10-25）

| # | 规则名称 | 优先级 | 匹配模式 | 脱敏示例 |
|---|---------|--------|---------|----------|
| 1 | 国际电话号码 | 10 | E.164 格式 | `+1-202-555-0123` → `+1-***-***-23` |
| 2 | 信用卡号 (Luhn) | 15 | Visa/MC/Amex/Discover + Luhn 校验 | `4111111111111111` → `****-****-****-1111` |
| 3 | IPv4 地址 | 20 | `xxx.xxx.xxx.xxx` | `192.168.1.100` → `***.***.***.100` |
| 4 | MAC 地址 | 20 | `XX:XX:XX:XX:XX:XX` 或 `XX-XX-XX-XX-XX-XX` | `00:1A:2B:3C:4D:5E` → `XX:**:**:**:**:5E` |
| 5 | IPv6 地址 | 21 | `{hex}:{hex}:...:{hex}` | `2001:db8::1` → `****:****:****:XXXX` |

#### 中优先级规则（30-40）

| # | 规则名称 | 优先级 | 匹配模式 | 脱敏示例 |
|---|---------|--------|---------|----------|
| 6 | 护照号 | 30 | `[A-Z]\d{6,9}` (G/E 开头) | `G12345678` → `******XX` |
| 7 | SSN (美国社保号) | 35 | `XXX-XX-XXXX` | `123-45-6789` → `***-**-6789` |
| 8 | DB 连接串 | 40 | `protocol://user:pass@host/db` | `postgres://admin:secret@...` → `postgres://***:***@***` |

#### 低优先级规则（50-100）

| # | 规则名称 | 优先级 | 匹配模式 | 脱敏示例 |
|---|---------|--------|---------|----------|
| 9 | GitHub Token | 50 | `ghp_/gho_/ghu_/ghs_/ghr_` 前缀 | `ghp_xxxx` → `***REDACTED_GITHUB***` |
| 10 | Slack Token | 51 | `xoxb-/xoxp-/xapp-` 前缀 | `xoxb-xxxx` → `***REDACTED_SLACK***` |
| 11 | Stripe Key | 52 | `sk_live_/rk_live_/pk_live_` 前缀 | `sk_live_xxxx` → `***REDACTED_STRIPE***` |
| 12 | Google API Key | 53 | `AIza` 前缀 35 字符 | `AIzaSy...` → `***REDACTED_GOOGLE***` |
| 13 | 私钥 PEM | 54 | `-----BEGIN ... PRIVATE KEY-----` | PEM 块 → `***REDACTED_PRIVATE_KEY***` |
| 14 | 邮箱地址 | 100 | 标准邮箱格式 | `user@example.com` → `**@**.***` |
| 15 | 电话号码 (中国) | 100 | `1[3-9]\d{9}` | `13812345678` → `***-****-****` |
| 16 | 身份证号 (中国) | 100 | 18 位独立字符串 | `110101199001011234` → `******1234` |
| 17 | 银行卡号 | 100 | 8+ 位连续数字 | `6222021234567890123` → `****-****-****-0123` |
| 18 | API 密钥 | 100 | `api_key=...` 格式 | `api_key=xxx` → `api_key=***REDACTED***` |
| 19 | AWS 凭据 | 100 | `AKIA/ABIA/ACCA/ASIA` 前缀 | `AKIAIOSFODNN7EXAMPLE` → `***REDACTED***` |
| 20 | JWT Token | 100 | `eyJ.eyJ.sig` 格式 | JWT → `***REDACTED_JWT***` |
| 21 | 通用密钥 | 100 | token/secret/key/password 等 | `secret=xxx` → `***REDACTED***` |

> 信用卡规则使用 **Luhn 算法**验证校验和，仅脱敏通过校验的合法卡号，避免误判普通数字序列。

### 字段级敏感检测

敏感字段名检测（29 种模式，字段名包含即命中）：

```rust
static SENSITIVE_FIELDS: &[&str] = &[
    // 认证凭据
    "password", "token", "secret", "key", "credential", "auth",

    // API 密钥
    "api_key", "api_key_id", "api_secret", "access_key", "access_key_id",
    "secret_key", "private_key", "public_key", "encryption_key",
    "decryption_key", "master_key", "session_key",

    // OAuth 相关
    "oauth", "oauth_token", "oauth_secret", "bearer", "bearer_token",
    "jwt", "session_id", "session_token",

    // AWS 凭据
    "aws_secret", "aws_key", "aws_credentials",

    // 数据库连接
    "database_url", "db_password", "db_user", "connection_string",

    // 支付和个人身份信息
    "credit_card", "card_number", "cvv", "ssn", "social_security",

    // 其他敏感信息
    "client_secret", "client_id", "refresh_token", "pin", "pin_code",
    "two_factor", "totp", "backup_code", "recovery_code",
];
```

快速检测函数：

```rust
impl DataMasker {
    pub fn is_sensitive_field(field_name: &str) -> bool {
        let lower_name = field_name.to_lowercase();
        SENSITIVE_FIELDS
            .iter()
            .any(|sensitive| lower_name.contains(*sensitive))
    }
}
```

### 自定义规则

通过 `MaskRuleBuilder`、`DataMaskerBuilder` 与 `MaskRuleRegistry` 灵活扩展：

```rust
use inklog::{DataMasker, MaskRule, MaskRuleBuilder, MaskRuleRegistry};

// 创建自定义规则
let custom_rule = MaskRule::builder("employee_id")
    .pattern(r"\bEMP-\d{6}\b")
    .replacement("EMP-***")
    .priority(30)
    .build()
    .expect("Invalid pattern");

// 在内置规则基础上组装脱敏器
let masker = DataMasker::builder()
    .add_rule(custom_rule)
    .disable_builtin("bank_card")  // 禁用特定内置规则
    .build();

let result = masker.mask("Employee: EMP-123456");
// 结果: "Employee: EMP-***"
```

使用 `MaskRuleRegistry` 管理规则：

```rust
use inklog::{MaskRule, MaskRuleBuilder, MaskRuleRegistry};

// 创建包含所有内置规则的注册中心
let mut registry = MaskRuleRegistry::with_builtins();

// 注册自定义规则
let custom = MaskRule::builder("custom_pattern")
    .pattern(r"CUSTOM-\d+")
    .replacement("***CUSTOM***")
    .priority(25)
    .build()
    .unwrap();
registry.register(custom).unwrap();

// 启用/禁用特定规则
registry.set_enabled("email", false);

// 获取当前活跃规则（按优先级排序）
let active = registry.active_rules();
```

从 TOML 配置加载规则：

```rust
let toml_config = r#"
[[masking_rules]]
name = "employee_id"
pattern = "\\bEMP-\\d{6}\\b"
replacement = "EMP-***"
priority = 30

[[masking_rules]]
name = "project_code"
pattern = "PRJ-[A-Z]{3}-\\d{4}"
replacement = "PRJ-***-XXXX"
priority = 35
"#;

let rules = MaskRuleRegistry::load_from_toml(toml_config).unwrap();
```

### 直接 API 使用

```rust
use inklog::masking::DataMasker;

let masker = DataMasker::new();

// 字符串脱敏
let masked = masker.mask("Contact admin@example.com at 13812345678");
// 结果: "Contact ***@***.*** at 138****5678"

// JSON 结构递归脱敏
use serde_json::json;

let mut data = json!({
    "email": "admin@company.org",
    "phone": "13912345678",
    "password": "secret123",
    "contacts": ["user1@test.com", "13811112222"]
});

masker.mask_value(&mut data);
// password 字段经敏感字段名检测被完全脱敏
```

### 配置启用

```toml
# inklog_config.toml
[global]
masking_enabled = true  # 启用数据脱敏（默认: true）
```

```bash
# 环境变量
export INKLOG_GLOBAL_MASKING_ENABLED=true
```

## 🧠 内存安全

inklog 使用 Rust 的内存安全保证和 `zeroize` 库，确保敏感数据在内存中不留痕迹。

### Zeroize 集成

加密密钥全程使用 `Zeroizing` 包装（见 `get_encryption_key`），离开作用域时自动清零：

```rust
use zeroize::Zeroizing;

// 环境变量值 / 派生密钥自动清零
let key = Zeroizing::new(std::env::var("INKLOG_ENCRYPTION_KEY")?);
// ... 使用密钥 ...
// key 离开作用域时自动清零内存
```

**清理保证**：

- **自动清理**：`Zeroizing` 在作用域结束时自动清零；
- **手动清理**：调用 `.zeroize()` 立即清理；
- **确定性覆盖**：覆盖内存而非仅释放，编译器优化不会跳过清理操作。

### unsafe 使用说明

- 生产代码路径几乎不使用 `unsafe`：唯一的 FFI 是 Windows 平台的磁盘空间查询（`GetDiskFreeSpaceExW`，用于 FileSink 磁盘空间管理）；
- 测试代码中存在少量 `unsafe` 块，用于满足 edition 2024 对 `std::env::set_var` / `std::env::remove_var` 的 unsafe 标记要求；
- 其余内存安全由 Rust 编译器保证：无 C 风格指针操作、无手动内存管理、无缓冲区溢出风险。

## 🚪 访问控制

inklog 在文件、数据库和网络层面实施严格的访问控制。

### 文件访问控制

**路径穿越防护**（`validation::path` PathValidator）：

- 校验日志路径不在系统目录、用户主目录与密钥文件等敏感位置；
- 解析 `%XX` 编码、反斜杠与控制字符，防止编码绕过；
- Unix 下以 `O_NOFOLLOW | O_CLOEXEC` + **0600 权限**创建校验过的日志文件，在内核层拒绝符号链接替换（关闭 validate-then-use 竞态）；非 Unix 平台退化为普通创建；
- CLI 解密工具对输入路径做规范化（canonicalize）并校验位于基础目录内，防止 `../../../etc/passwd` 类攻击。

**磁盘空间管理**（FileSink）：

- 可用空间不足（< 5% 或 < 100MB）时告警并降级；
- 自动清理旧日志（按 `retention_days` / `max_total_size` 策略）。

### 数据库访问控制

**SQL 注入防护**：

- 表名白名单校验（`validate_table_name`）：仅允许字母、数字、下划线，必须以字母或下划线开头，长度 ≤ 128；
- 业务数据一律经参数化查询写入，禁止字符串拼接；
- DDL（建表/分区）使用校验后的名称构建。

**连接池管理**：

```rust
let mut opt = ConnectOptions::new(url);
opt.max_connections(pool_size)                 // 最大连接数限制
   .min_connections(2)                         // 最小连接数
   .connect_timeout(Duration::from_secs(5))    // 连接超时
   .idle_timeout(Duration::from_secs(8));      // 空闲超时
```

- **连接数限制**：防止资源耗尽攻击；
- **超时保护**：避免长时间挂起的连接；
- **连接复用**：减少认证开销。

**最小权限原则**（数据库用户建议）：

```sql
-- PostgreSQL: 创建专用日志用户并授予最小权限
CREATE USER inklog_writer WITH PASSWORD 'secure_password';
GRANT CONNECT ON DATABASE logs TO inklog_writer;
GRANT USAGE ON SCHEMA public TO inklog_writer;
GRANT SELECT, INSERT ON ALL TABLES IN SCHEMA public TO inklog_writer;
GRANT CREATE ON SCHEMA public TO inklog_writer;  -- 分区创建（如需要）
```

```sql
-- MySQL
CREATE USER 'inklog_writer'@'%' IDENTIFIED BY 'secure_password';
GRANT INSERT, SELECT ON logs.* TO 'inklog_writer'@'%';
GRANT CREATE ON logs.* TO 'inklog_writer'@'%';  -- 分区创建
```

**SQLite**：文件系统权限控制（0600），仅本地访问。

### HTTP 端点访问控制（`http` feature）

| 机制 | 说明 |
|------|------|
| **认证 token** | 经环境变量注入，启动期缓存，获取失败 fail-closed（不再降级为无认证） |
| **IP 白名单** | `ip_whitelist` 限制访问来源 |
| **TLS** | `TlsConfig`（cert_path / key_path，rustls）启用 HTTPS |
| **错误模式** | `Strict` 启动失败即返回错误 / `Warn` 记录警告并继续 |

## 🌐 网络安全

inklog 在数据库与网络转发通信中实施严格的网络安全措施。

### 数据库安全连接

**PostgreSQL SSL 模式**：

```rust
let url = "postgres://user:pass@localhost/logs?sslmode=require".to_string();
// sslmode 选项:
// - disable: 禁用 SSL (不推荐)
// - allow: 优先 SSL,失败则不加密
// - prefer: 优先 SSL,失败则明文
// - require: 必须使用 SSL (推荐)
// - verify-ca: 验证 CA 证书
// - verify-full: 验证 CA 和主机名 (最安全)
```

**MySQL SSL**：

```rust
let url = "mysql://user:pass@localhost/logs?ssl_mode=REQUIRED".to_string();
```

**SQLite**：本地文件访问，文件权限控制。

**连接超时**：连接超时 5 秒、空闲超时 8 秒、连接最大生存时间 1 小时、获取连接超时 30 秒。

### 网络转发 Sink 安全（`net-sink` feature）

- **TCP**：可选 TLS（rustls 客户端），断线缓冲与半开探测；
- **UDP**：NDJSON 数据报，适用于低敏感度、高吞吐场景；
- 敏感日志转发建议优先 TCP + TLS。

### 防火墙规则建议

```bash
# PostgreSQL pg_hba.conf (仅允许应用服务器访问)
hostssl logs    inklog_writer    10.0.1.0/24    scram-sha-256
```

## 📜 合规性

inklog 的安全设计支持多种合规性要求，包括 GDPR、HIPAA 和 PCI-DSS。

### GDPR（通用数据保护条例）

#### 个人数据处理

PII 数据自动脱敏覆盖以下类型：

- ✅ 电子邮箱地址（邮箱脱敏）
- ✅ 电话号码（中国手机号 + 国际 E.164 格式）
- ✅ 身份标识符（中国身份证、美国 SSN、护照号）
- ✅ 网络标识符（IPv4/IPv6 地址、MAC 地址）
- ✅ 金融凭据（信用卡号 + Luhn 校验、银行卡号）
- ✅ 第三方令牌（GitHub/Slack/Stripe/Google API Key）
- ✅ 私钥 PEM 证书
- ✅ 数据库连接串

#### 数据主体权利

| 权利 | inklog 支持手段 |
|------|------------------|
| **被遗忘权** | 文件端：`retention_days` / `max_total_size` / `cleanup_interval_minutes` 自动清理过期日志；数据库端：按分区策略删除或按时间过滤删除过期记录 |
| **数据访问与携带权** | 经 `inklog-cli query` 按时间/级别/关键词检索导出日志；数据库端按时间范围查询（结构化字段存储于 `fields` JSON） |
| **数据最小化** | 全局级别控制（如 `warn` 级别不记录 DEBUG/TRACE）+ PII 自动脱敏 |

#### 数据保护措施

- ✅ **加密存储**（GDPR 第 32 条）：AES-256-GCM 静态数据加密；
- ✅ **传输加密**：HTTPS/TLS（HTTP 端点、TCP Sink、数据库 SSL）；
- ✅ **密钥管理服务集成**（`kms` feature）。

### HIPAA（健康保险流通与责任法案）

#### 受保护健康信息（PHI）保护

PHI 字段（如患者 ID、诊断、用药）可通过自定义脱敏规则覆盖：

```rust
use inklog::MaskRule;

// 为 PHI 字段注册自定义脱敏规则
let patient_rule = MaskRule::builder("patient_id")
    .pattern(r"\bPID-\d{8}\b")
    .replacement("PID-***")
    .priority(30)
    .build()
    .expect("Invalid pattern");
```

#### 审计与技术保障

**审计事件**（经 `publish_ops_event` 与结构化日志）：

- 敏感数据访问
- 密钥使用
- 日志导出操作
- 加密/解密操作

**技术保障措施**：

| 类别 | 措施 |
|------|------|
| **访问控制** | 最小权限数据库用户、HTTP 认证 + IP 白名单 |
| **传输安全** | TLS 1.2+ 数据库连接与 TCP Sink |
| **物理和环境安全** | 网络隔离（部署侧配合） |
| **完整性** | 归档 HMAC-SHA256 防篡改链（ArchiveChain） |

### PCI-DSS（支付卡行业数据安全标准）

#### 支付卡数据保护

```rust
// ✅ 自动脱敏信用卡号（Luhn 校验通过的卡号才脱敏）
log::info!("Payment processed: card_number=6222021234567890123");
// 输出: Payment processed: card_number=****-****-****-0123
```

#### PCI-DSS 合规要求映射

| 要求 | inklog 实现 |
|------|-------------|
| **禁止存储完整卡号** | 信用卡号自动脱敏（仅保留尾 4 位） |
| **加密传输** | HTTPS/TLS |
| **加密存储** | AES-256-GCM |
| **访问控制** | 文件权限 0600、数据库最小权限、HTTP 认证 |
| **日志监控** | 结构化日志 + Prometheus 指标 + 归档防篡改链 |
| **定期漏洞扫描** | `cargo deny check` + `cargo audit`（CI 与 pre-push） |

#### 支付日志最佳实践

**不记录的敏感信息**：

- ❌ CVV/CVC 码
- ❌ PIN 码
- ❌ 完整磁条数据
- ❌ 完整卡号（仅记录后 4 位）

```rust
// ✅ 正确的做法
log::info!("Payment authorized: card_last4=0123, amount=100.00");

// ❌ 错误的做法（将被自动脱敏兜底，但不应依赖）
log::info!("Payment: card_number=6222021234567890123, cvv=123");
```

### 合规性检查清单

**GDPR**：

- [ ] 启用数据脱敏（`INKLOG_GLOBAL_MASKING_ENABLED=true`）
- [ ] 配置数据保留期限（`retention_days`）
- [ ] 实施加密日志（`encrypt = true`）
- [ ] 定期清理过期日志（`cleanup_interval_minutes` / 数据库分区清理）
- [ ] 记录数据访问审计日志（启用 `tracing` debug 级别）

**HIPAA**：

- [ ] 限制数据库访问权限（专用用户）
- [ ] 启用审计日志（tracing debug 级别）
- [ ] 实施 PHI 字段脱敏（自定义规则）
- [ ] 配置网络隔离

**PCI-DSS**：

- [ ] 启用信用卡号脱敏（内置规则，默认启用）
- [ ] 不记录 CVV/CVC 码（代码审查）
- [ ] 使用 TLS 1.2+ 数据库连接
- [ ] 定期运行安全扫描（`cargo deny check` / `cargo audit`）
- [ ] 配置日志轮转和归档

## ✅ 安全最佳实践

### 1. 密钥管理

**推荐做法**：

```bash
# 使用 .env 文件（不要提交到 git）
echo "INKLOG_ENCRYPTION_KEY=$(openssl rand -base64 32)" >> .env
echo ".env" >> .gitignore
```

- 定期密钥轮换（建议 90 天，流程见上文「密钥轮换」）；
- 高安全环境启用 `kms` feature（Vault transit MVP）；
- 不要在日志中输出密钥，不要使用弱密钥（低熵密钥会被启动校验拒绝）。

### 2. 文件和目录安全

```bash
# 敏感文件 0600，目录 0700（inklog 创建的校验日志文件已默认 0600）
chmod 600 /etc/inklog/config.toml
chmod 700 /var/log/inklog/
```

**安全目录结构**：

```text
/var/log/inklog/
├── app.log                     # 活动日志
├── app_20260913_143022.log.zst.enc  # 加密归档
└── backup/                     # 轮换备份
    └── 2026-09/
```

### 3. 数据库安全

- 使用专用最小权限用户（见「数据库访问控制」）；
- 强制 SSL 连接（`sslmode=verify-full` / `ssl_mode=REQUIRED`）；
- 启用连接超时与连接数限制。

### 4. 日志配置安全

```toml
[global]
level = "info"           # 生产环境不记录 DEBUG/TRACE
masking_enabled = true   # 默认启用

[file_sink]
encrypt = true           # 启用轮转归档加密
retention_days = 30      # 限制保留期限
```

### 5. 依赖安全

```bash
cargo deny check advisories   # 已知漏洞
cargo deny check bans         # 重复依赖
cargo deny check licenses     # 许可证
cargo audit                   # RustSec 公告
```

应用侧建议提交 `Cargo.lock` 以锁定确切版本。

### 6. 监控和审计

```rust
use inklog::LoggerManager;

let logger = LoggerManager::with_config(config).await?;

// 定期检查日志系统健康状态
let health = logger.get_health_status();
if let Some(file_health) = health.sinks.get("file") {
    if !file_health.status.is_operational() {
        eprintln!("WARNING: File sink is unhealthy!");
    }
}
```

生产环境建议启用 `http` feature，经 Prometheus 抓取 `inklog_sink_healthy` 等指标持续监控。

### 7. 故障处理

- 数据库故障时自动按 DB → File → Console 三级降级，恢复由健康检查线程自动完成（见 [🏗️ 架构设计](ARCHITECTURE.md)「故障降级与自愈」）；
- 错误消息不记录密钥或敏感数据（i18n 渲染的固定消息模板 + 脱敏管线兜底）。

## 📎 附录

### A. 完整安全配置示例（inklog_config.toml）

```toml
[global]
level = "info"
format = "{timestamp} [{level}] {target} - {message}"
masking_enabled = true  # 启用数据脱敏

[console_sink]
enabled = true

[file_sink]
enabled = true
path = "logs/app.log"
max_size = "100MB"
rotation_time = "daily"
keep_files = 30
compress = true
compression_level = 3
encrypt = true                            # 启用加密
encryption_key_env = "INKLOG_ENCRYPTION_KEY"
retention_days = 30
max_total_size = "1GB"
cleanup_interval_minutes = 60

[database_sink]
enabled = true
driver = "postgres"
url = "postgres://inklog_writer:password@localhost/logs?sslmode=require"
pool_size = 10
batch_size = 100
flush_interval_ms = 1000
table_name = "logs"
```

**环境变量示例（.env，不入库）**：

```bash
# 全局配置
INKLOG_GLOBAL_LEVEL=info
INKLOG_GLOBAL_MASKING_ENABLED=true

# 文件加密
INKLOG_ENCRYPTION_KEY=$(openssl rand -base64 32)

# 数据库连接
INKLOG_DATABASE_SINK_URL=postgres://inklog_writer:${DB_PASSWORD}@localhost/logs?sslmode=require
INKLOG_DATABASE_SINK_POOL_SIZE=10

# 解密密钥
INKLOG_DECRYPT_KEY=$INKLOG_ENCRYPTION_KEY
```

### B. CLI 安全命令速查

```bash
# 生成配置模板
inklog-cli generate --config-type full --output ./config/

# 验证配置
inklog-cli validate --config ./config/inklog_config.toml

# 解密日志文件
inklog-cli decrypt \
  --input logs/encrypted.log.zst.enc \
  --output logs/decrypted.log \
  --key-env INKLOG_DECRYPT_KEY

# 批量解密目录（递归）
inklog-cli decrypt \
  --input logs/ \
  --output decrypted/ \
  --batch \
  --recursive \
  --key-env INKLOG_DECRYPT_KEY

# 检索本地日志（自动解密解包）
inklog-cli query --path logs/ --level error --since 2026-09-13T00:00:00Z
```

### C. 安全检查命令

```bash
# 1. 运行测试（CI 口径，数据库后端互斥不适用 --all-features）
cargo test --workspace --features "sqlite http cli kit compression gzip parquet fast-masking test-utils"

# 2. Clippy 与格式检查
cargo clippy --all-targets --features "sqlite http cli kit compression gzip parquet fast-masking" -- -D warnings
cargo fmt --all -- --check

# 3. 安全审计
cargo deny check advisories
cargo deny check bans
cargo deny check licenses
cargo audit

# 4. 代码覆盖率（门禁 ≥ 80%）
cargo llvm-cov --features "sqlite http cli kit compression gzip parquet fast-masking" --lib --fail-under-lines 80

# 5. 检查日志文件权限
find logs/ -type f -exec chmod 600 {} \;
find logs/ -type d -exec chmod 700 {} \;
```

### D. 术语表

| 术语 | 定义 |
|------|------|
| **AES-256-GCM** | 高级加密标准 256 位，使用伽罗瓦/计数器模式 |
| **Nonce** | 密码学随机数，用于加密过程中的唯一性 |
| **PBKDF2** | 基于密码的密钥派生函数（inklog 使用 HMAC-SHA256，600,000 次迭代） |
| **Zeroize** | 安全清零内存的技术 |
| **PII** | 个人身份信息 (Personally Identifiable Information) |
| **PHI** | 受保护健康信息 (Protected Health Information) |
| **KMS** | 密钥管理服务 (Key Management Service) |
| **TLS** | 传输层安全协议 (Transport Layer Security) |
| **GDPR** | 通用数据保护条例 (General Data Protection Regulation) |
| **HIPAA** | 健康保险流通与责任法案 (Health Insurance Portability and Accountability Act) |
| **PCI-DSS** | 支付卡行业数据安全标准 (Payment Card Industry Data Security Standard) |

### E. 常见安全问题

**问题 1**：解密失败 "Authentication failed"

**原因**：密钥不匹配或文件损坏。

**解决方案**：

```bash
# 验证密钥环境变量内容（长度应为 44 的 Base64 串）
echo -n "$INKLOG_DECRYPT_KEY" | wc -c

# 确认使用与加密时一致的密钥
export INKLOG_DECRYPT_KEY=$INKLOG_ENCRYPTION_KEY
```

**问题 2**：文件权限错误 "Permission denied"

**解决方案**：

```bash
# 修复文件权限
chmod 600 logs/*.enc
chown $(whoami):$(whoami) logs/*.enc
```

**问题 3**：启动时报密钥被拒绝

**原因**：密钥不是 32 字节（Base64 解码后）或熵值过低。

**解决方案**：使用 `openssl rand -base64 32` 重新生成密钥。

---

**文档版本**: 2.1
**维护者**: inklog Security Team
**联系我们**: [Kirky-X@outlook.com](mailto:Kirky-X@outlook.com)

*本文档遵循 CC BY-SA 4.0 许可协议*
