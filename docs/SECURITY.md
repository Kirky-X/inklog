# 🔒 inklog 安全文档

本文档描述 inklog 的安全设计（加密、数据脱敏、内存安全、访问控制、网络安全与合规性）、漏洞报告流程，以及使用 inklog 处理敏感日志时的安全最佳实践。

> 相关文档：[📖 用户指南](USER_GUIDE.md) · [🏗️ 架构设计](ARCHITECTURE.md) · [🤝 贡献指南](CONTRIBUTING.md)

<details open>
<summary>📑 目录</summary>

- [📌 支持的版本与维护策略](#-支持的版本与维护策略)
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

## 📌 支持的版本与维护策略

本项目处于 0.x 预发布阶段，**仅维护当前 minor 版本线**（`0.3.x`，含该线内的 rc 预发布版本）。

- **当前线**：接收缺陷修复与安全修复，修复随该线的最新发布版本提供。
- **旧 minor 线**：在下一个 minor 线的首个版本发布时即进入 EOL，不再提供任何修复，安全补丁也不回移。
- **不设 LTS**：1.0 之前不承诺跨 minor 的 API 兼容性，minor 位升级可能包含破坏性变更，升级前请阅读 [CHANGELOG](CHANGELOG.md)。
- **crates.io 可见性 ≠ 受支持**：旧版本保持可下载（除非因严重缺陷执行 yank），但不属于受支持范围。

| 版本线 | 状态 | 说明 |
|--------|------|------|
| `0.3.x`（当前） | ✅ 支持 | 唯一接收修复的版本线 |
| `≤ 0.2.x` | ❌ EOL | 已被当前线取代，不再维护，请升级 |

> 历史安全修复示例：0.1.11 修复了 `table_name` SQL 注入防护、`FileSink` 路径穿越防护与 HTTP 认证 fail-closed 三项安全问题（详见 [更新日志](CHANGELOG.md)）；该线已 EOL，不会获得后续修复。

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

- [GitHub Security Advisories](https://github.com/Kirky-X/inklog/security/policy)（仓库页 Security → Report a vulnerability）

**备选方式**：

- 邮件：[Kirky-X@outlook.com](mailto:Kirky-X@outlook.com)（受理人：Kirky.X；未发布 PGP 公钥，请勿以加密邮件预期）

### 报告内容

| 部分 | 应包含的信息 |
|------|--------------|
| **漏洞描述** | 漏洞类型（注入、认证绕过等）、影响范围（受影响版本）、严重程度（CVSS 评分） |
| **复现步骤** | 详细的重现步骤、代码示例或配置、预期行为与实际行为 |
| **影响评估** | 数据泄露可能性、系统可用性影响、业务影响范围 |
| **缓解措施** | 临时缓解方案、建议的修复方向 |

### ⏱️ 响应承诺

> 本仓响应时限严于七仓基线（基线：确认 48 小时内、初步评估 7 天内），以本表为准。

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

### 已知安全问题与订阅

查看当前已知安全问题：

- **GitHub Security Advisories**：<https://github.com/Kirky-X/inklog/security/policy>
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
| 数据脱敏 | `support::processing::masking` | 21 条内置正则规则（fancy-regex 环视实现 CJK 友好边界）+ 敏感字段名检测 + 自定义注册表；console/file/database/net/otlp 全部出口默认掩码（`masking_enabled` 门控），递归深度 16 层上限、单条 1 MiB 上限 |
| Secret 出站脱敏门 | `support::processing::secret_scan`（`secret-scan` feature） | 值形态正则注册表（`sk-` 前缀/PEM 私钥块/AWS AKIA 等九类裸 secret）+ 值熵扫描（Shannon，阈值可配）+ 逐模式计数；`mask_checked` 超限 fail-closed 拒发输出；`mask` 对超大输入（>1 MiB）以整体遮蔽收口，同样 fail-closed（不扫描即不可证安全）；中文敏感键名组并入键名检测 |
| 路径安全 | `validation::path`（PathValidator） | 路径穿越防护，禁止写入用户主目录与密钥文件 |
| 内容净化 | `validation::sanitize`（LogSanitizer） | 日志注入与控制字符防护 |
| SQL 注入防护 | DatabaseSink / `integrations::infra::database` | 表名白名单校验 + 参数化查询 |
| HTTP 访问控制 | `http` feature | 认证 token 启动期缓存、失败 fail-closed；IP 白名单；TLS；非回环绑定且无鉴权/白名单时**配置期拒绝**（暴露面守卫覆盖 `InklogConfig::validate` 与库自身的构建入口） |
| 错误文本出口脱敏 | `error::InklogError` / `validation::sanitize` | `Display` / `Debug` / `localized_message()` / `safe_message()` 全部出口脱敏——路径、DSN、SQL 片段与凭据不随错误文本外带；`LogRecord` 的 `Debug` 视图经脱敏，`Serialize` 保持原文（落盘脱敏是 sink 的职责，由 `masking_enabled` / `secret_scan_enabled` 门控） |
| 出站 TLS 认证 | `support::io::sink::net` / `otlp`（`net-sink` / `otlp` feature） | `danger_accept_invalid_certs`（跳过证书校验）受**编译期门控**：仅调试构建（`debug_assertions`）或显式启用 `insecure-tls` feature 可用，发布构建无授权即构造失败（fail-closed）；放行时告警走标准错误（sink 构造期全局 subscriber 尚未安装，`tracing::warn!` 必被丢弃） |
| 归档防篡改 | `support::audit_chain`（ArchiveChain） | HMAC-SHA256 归档链（随机链首盐），防删除、重排与伪造；`audit_chain_enabled = true` 后每次轮转自动登记 `{path, sha256, timestamp}` 并写穿 `<stem>.chain.jsonl` manifest 与 `<stem>.chain.seal.json` 封条（链外锚点：条目总数 + 尾哈希 + 封条 HMAC），`inklog-cli verify-chain --manifest <file> --key-env INKLOG_AUDIT_KEY` 校验（链完整退出码 0、篡改退出码 2；**空 manifest 与封条缺失均判为失败**——链内自洽性校验无法证明「没有条目被删掉」，空链与无锚点一律按 fail-closed 处理；**截尾到非空前缀由封条检出**，链内校验检不出截尾） |
| 供应链安全 | `deny.toml` / lefthook | `cargo deny check` + `cargo audit` + pre-commit 私钥扫描 |

> 说明：仓库维护 [`deny.toml`](../deny.toml)，CI 的 security job 与 lefthook pre-push 分别运行 `cargo deny check`（漏洞/许可证/重复依赖）与 `cargo audit`（RustSec 公告）。

### 归档审计链的威胁模型边界

- **封条与 manifest 同目录同可写域**：封条路径（`<stem>.chain.seal.json`）由 manifest 路径推导，写入侧与 manifest 分两次落盘（非原子成对发布）。把某一代的 manifest 与当时合法的封条一并回滚，对 `inklog-cli verify-chain` 不可见——**封条只挡单侧截尾，不挡成对回滚**；覆盖成对回滚需把封条放到独立可写域（当前 CLI 无 `--seal` 参数）。
- **校验期间的轮转**：manifest 与封条是两次独立读取，轮转落在两次读之间即产生条目数错配。CLI 侧重读一次再判定——重读后配对一致即判「校验期间发生轮转」并按重读后的实际结果给退出码；重读后仍不一致则输出与「篡改」区分开的诊断（manifest 与封条不同代），退出码仍为 2（fail-closed，不把写入侧不一致误报为人为篡改）。
- **崩溃残留**：写入侧在 manifest 与封条之间崩溃会留下错配，直到下一次成功轮转把两者再次写齐为止——期间校验按 fail-closed 处理。

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

**密钥文件（次要来源）**：环境变量未设置时可从密钥文件读取（`encryption_key_file` / journal 的 `fallback_journal_key_file`），解析优先级 **env > 文件**。密钥文件 unix 权限必须 `0600`——组/其他可读的密钥文件在解析期显性拒绝；两个来源均不可用时加密路径**显性失败**（构建期报错），不存在静默明文降级。

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

### fallback journal 加密（可选）

`global.fallback_journal_encrypt = true` 时，磁盘持久化兜底 journal 以 AES-256-GCM 加密落盘：

```text
偏移      大小     描述
--------  -------  ------------------
0-7       8 字节   MAGIC: "INKJRN1\0"
8-9       2 字节   格式版本: 1 (u16 little-endian)
10-11     2 字节   算法标识: 1 (AES-256-GCM)
12-27     16 字节  PBKDF2 盐
其后      逐段     nonce(12) + 段长(4, u32le) + 密文（明文为单行 JSON）
```

- **nonce 唯一性策略**：随机 96-bit/段（CSPRNG），同 key 跨段/跨文件不重复由单测断言；
- **显性失败**：加密启用而 key 缺失/无效（env 与 key 文件均不可用、格式不符、原始密钥熵不足）时构建期报错，禁止静默明文落盘；
- **格式自适应**：`replay` 按 magic 识别新旧格式——旧明文 JSONL 可被加密实例重放；不可识别/不可解密（版本或算法不支持、明文实例遇加密文件的配置回滚、错误密钥全军覆没）时文件原样保留并按段计数上报，不做静默清空销毁；
- **文件权限**：加密 journal 的头创建、段追加与截断重建全部以 unix `0600` 创建（对齐 FileSink 加密产物与 key 文件约束）；**明文 journal 同权限语义**——明文路径不再按默认 umask 建 0644，落盘后显式收紧既有文件的权限（`mode` 仅在创建时生效）；journal 父目录新建时收敛为 `0700`；
- **明文模式的威胁模型**：`fallback_journal_encrypt = false`（默认）时兜底 journal 以**明文 JSONL** 落盘——其中集中存放 ERROR/FATAL 级记录，敏感上下文（异常消息、字段透传）可能比主日志更密集；共享主机/备份链路场景下该文件与主日志同等暴露，敏感部署应启用加密或将 `fallback_journal_path` 指向受限目录（文件权限已统一为 0600、目录 0700，目录权限不改变文件本身的暴露面）；
- **推送端口**：`fallback_journal_push_addr` 可选把 spill 记录同步推送远端（断线缓冲、失败计数不反压落盘）；`TransportSecurity` 预留 TLS 扩展点，当前为明文出站，部署在可信网段或由隧道层提供传输安全。

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
| 1 | 国际电话号码 | 10 | E.164 格式 | `+1-202-555-0123` → `+1-***-***-0123` |
| 2 | 信用卡号 (Luhn) | 15 | Visa/MC/Amex/Discover + Luhn 校验 | `4111111111111111` → `****-****-****-1111` |
| 3 | IPv4 地址 | 20 | `xxx.xxx.xxx.xxx` | `192.168.1.100` → `***.***.***.100` |
| 4 | MAC 地址 | 19 | `XX:XX:XX:XX:XX:XX` 或 `XX-XX-XX-XX-XX-XX` | `00:1A:2B:3C:4D:5E` → `00:**:**:**:**:5E` |
| 5 | IPv6 地址 | 21 | 完整 8 组形式与 `::` 压缩形式（RFC 5952） | `2001:0db8:0000:0000:0000:0000:0000:0001` → `****:****:****:****:****:****:****:0001`；`2001:db8::1` → `****:****:****:****:****:****:****:1` |

> IPv6 规则同时命中完整分组形式与 `::` 压缩写法（含 `::1`、`1::`、裸 `::`）；命中时统一输出 8 组形态，末段分组保留真实值（无末段的压缩写法该组同为 `****`），其余分组以 `****` 替换。IPv4 映射形式（如 `::ffff:192.0.2.1`）的点分尾段由 IPv4 规则先脱敏，`::ffff` 前缀段由本规则脱敏。

#### 中优先级规则（30-40）

| # | 规则名称 | 优先级 | 匹配模式 | 脱敏示例 |
|---|---------|--------|---------|----------|
| 6 | 护照号 | 30 | `[EeGg]\d{8}`（E/G 开头 + 8 位数字） | `G12345678` → `G******78` |
| 7 | SSN (美国社保号) | 35 | `XXX-XX-XXXX` | `123-45-6789` → `***-**-6789` |
| 8 | DB 连接串 | 40 | `protocol://user:pass@host/db` | `postgres://admin:secret@localhost/db` → `postgres://admin:***@localhost/db`（仅密码段被替换） |

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
| 17 | 银行卡号 | 100 | 16-19 位连续数字（4+8..11+4） | `6222021234567890123` → `****-****-****-0123` |
| 18 | API 密钥 | 100 | `api[_-]?key=...` 格式（值 ≥ 20 位） | `api_key=abcdefghijklmnopqrstuvwxyz` → `api_key=***REDACTED***`（键名保留，值整体替换为单标记） |
| 19 | AWS 凭据 | 100 | `AKIA/ABIA/ACCA/ASIA` 前缀 | `AKIAIOSFODNN7EXAMPLE` → `***REDACTED***` |
| 20 | JWT Token | 100 | `eyJ.eyJ.sig` 格式 | JWT → `***REDACTED_JWT***` |
| 21 | 通用密钥 | 100 | token/secret/key/password 等（值 ≥ 16 位） | `secret=abcdefghijklmnop` → `secret=***REDACTED***` |

> 信用卡规则使用 **Luhn 算法**验证校验和，仅脱敏通过校验的合法卡号，避免误判普通数字序列。

### 字段级敏感检测

敏感字段名检测（15 条词边界正则，大小写不敏感；启用 `secret-scan` 时追加 1 条中文键名族）：

```rust
static SENSITIVE_FIELD_PATTERNS: LazyLock<Vec<regex::Regex>> = LazyLock::new(|| {
    vec![
        // 认证凭据
        regex::Regex::new(r"(?i)\b(password|passwd|pwd)\b").unwrap(),
        // token/bearer/auth：前置字符须为分隔符或行首（避免 "cakey" 类误判）
        regex::Regex::new(r"(?i)(?:[^a-zA-Z0-9_])?(token|bearer|auth)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(secret|credential)\b").unwrap(),
        // API 密钥
        regex::Regex::new(r"(?i)\b(api[_-]?key|apikey|api[_-]?secret)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(access[_-]?key|access[_-]?key[_-]?id)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(secret[_-]?key|private[_-]?key|public[_-]?key)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(encryption[_-]?key|decryption[_-]?key|master[_-]?key)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(session[_-]?key|session[_-]?id|session[_-]?token)\b").unwrap(),
        // OAuth 相关
        regex::Regex::new(r"(?i)\b(oauth|oauth[_-]?token|oauth[_-]?secret)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(jwt(_[a-zA-Z0-9]+)?|bearer[_-]?token)\b").unwrap(),
        // AWS 凭据
        regex::Regex::new(r"(?i)\b(aws[_-]?secret|aws[_-]?key|aws[_-]?token|aws[_-]?credentials)\b").unwrap(),
        // 数据库连接
        regex::Regex::new(r"(?i)\b(database[_-]?url|db[_-]?password|db[_-]?user|connection[_-]?string)\b").unwrap(),
        // 支付和个人身份信息
        regex::Regex::new(r"(?i)\b(credit[_-]?card|card[_-]?number|cvv|ssn|social[_-]?security)\b").unwrap(),
        // 客户端凭据
        regex::Regex::new(r"(?i)\b(client[_-]?secret|client[_-]?id)\b").unwrap(),
        // 其他敏感信息
        regex::Regex::new(r"(?i)\b(refresh[_-]?token|pin|pin[_-]?code|two[_-]?factor|totp|backup[_-]?code|recovery[_-]?code)\b").unwrap(),
    ]
});
```

启用 `secret-scan` feature 时追加第 16 条：`(密钥|令牌|密码|口令|凭证|凭据|私钥)$`（键名以敏感词收尾即视为敏感字段）。

快速检测函数：

```rust
impl DataMasker {
    pub fn is_sensitive_field(field_name: &str) -> bool {
        SENSITIVE_FIELD_PATTERNS
            .iter()
            .any(|pattern| pattern.is_match(field_name))
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
// 结果: "Contact **@**.*** at ***-****-****"

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

### Secret 出站脱敏门（feature `secret-scan`）

键名触发识别不了**无键名上下文的裸 secret**（日志正文里的 `sk-` 密钥、PEM 私钥块、AWS AKIA 形态等）。`secret-scan` feature（默认关闭）提供三层补充能力，经 `DataMasker` 挂到 sink 写出前的掩码点：

- **值形态注册表** `SecretPatternRegistry`：九类内置形态正则（前缀 + 长度 + 字符集三要素），可注册自定义形态；
- **值熵扫描** `EntropyScanner`：对 base64 核心字母 token（默认 ≥20 字符）做 Shannon 熵判定（阈值可配，默认 4.5 bits/char；hex 全集熵上限 4.0，哈希/UUID 族不误伤）。与 `encryption::validate_key_entropy` 分域——那边校验加密密钥字节，这边探测日志内容；
- **逐模式计数 + fail-closed** `SecretScanGate`：每条模式累计命中数可查（`hit_counts`）；`mask_checked` 在单条输入内任一模式命中数超门限时返回 `Err(InklogError::SecretScanLimit)` 拒发输出，`mask` 对超限输入整体遮蔽为 `***REDACTED_OVERSIZED***`（向后兼容）。

```rust
use inklog::{DataMasker, EntropyScanner, SecretPatternRegistry, SecretScanGate};

let gate = SecretScanGate::new(SecretPatternRegistry::with_builtins())
    .with_entropy(EntropyScanner::new())
    .with_match_limit(64);
let probe = gate.clone();

// 注入 sink 出站掩码点（console/file/net/ring_buffered/database 均经
// with_masker 挂接；database 自 0.3.0-rc.6 起支持）
let masker = DataMasker::builder().with_secret_scan(gate).build();

let out = masker.mask_checked("token=sk-proj-abcdefghijklmnopqrstuvwxyz123456")?;
assert_eq!(probe.hit_count("openai_sk"), Some(1));
```

启用该 feature 后，`DataMasker` 键名检测同时覆盖中文敏感键名（以 `密码`/`令牌`/`密钥`/`口令`/`凭证`/`凭据`/`私钥` 结尾的键，如 `数据库密码`、`访问令牌`；后缀锚定排除 `密码学` 等一般词汇）。

#### 接线边界与语义边界（必读）

- **声明式启用（0.3.0-rc.6 起）**：`console_sink.secret_scan_enabled` 与
  `database_sink.secret_scan_enabled`（均默认 `false`）在内置 sink 组装路径
  挂接 `secret_scan_masker()`（内置 PII 规则 + 出站门），无需再手工组装
  自定义 sink；builder 对应 `LoggerBuilder::console_secret_scan(bool)` /
  `LoggerBuilder::database_secret_scan(bool)`。`secret-scan` feature 关闭时
  该开关被忽略并在 validate 阶段发警告。注意 console 侧开关与
  `pii_masking_enabled` 叠加生效（PII 关则整体无掩码）；database 侧此前
  无 `with_masker` 注入口、masker 写死无门——本版本补齐（默认行为逐位
  不变）。
- **sink 出站掩码 fail-closed**：库内全部 sink 出站路径（database/console/file/ring_buffered_file/net/otlp）调用 `DataMasker::mask()`——命中照常替换；**不可证安全的情形不再放行原文**：
  - 超大输入（> 1 MiB）跳过扫描后以 `***REDACTED_OVERSIZED***` 整体遮蔽（不扫描即不可证安全，原样放行会让其中的凭据/PII 明文落盘）；
  - 正则引擎错误（`fancy-regex` 回溯预算耗尽等）以 `***REDACTED_ENGINE_ERROR***` 整体遮蔽并 warn 留痕（引擎错误下「该规则是否命中」不可判定）；
  - 脱敏标记不再让整条消息免于规则：标记保护下沉到**值**一级（值已是 `***REDACTED***`/`***MASKED***`/`[REDACTED]` 时不重复脱敏），消息中并存的裸 secret 与 PII 照常遮蔽——伪造标记无法豁免整条记录。
  `mask_checked()` 仍提供更严格的 fail-closed 出口（命中数超门限或超限时返回 `Err`，消费方显式选择）。
- **超大输入（> 1 MiB）两出口分叉**：`mask` 跳过扫描并整体遮蔽（留 warn——21 趟线性扫描对超大文本是 CPU 放大器）；`mask_checked` 返回 `Err(InklogError::SecretScanOversizedInput)` 拒绝输出——不扫描即不可证安全，fail-closed 出口不豁免。
- **脱敏标记不再关闭检测面**：门（`SecretScanGate`）与 `DataMasker::mask()` 均不对 `***REDACTED` 等标记做**整条消息**级短路——标记文本本身不被内置模式命中（幂等自然成立），同条消息中的裸 secret 照常替换与计数，伪造标记无法豁免整条记录。标记保护只作用于**值**一级（值已是标记时不重复脱敏，防止 REDACTED 套 REDACTED）。依赖门检测标记旁内容的调用方也可直接使用 `SecretScanGate::mask`/`mask_checked` 或 `DataMasker::detect`（检测面从不短路）。
- **自定义模式的 ReDoS 责任**：门跑在每条出站记录上，底层 `fancy-regex` 为回溯引擎；`SecretPatternRegistry::register` 不做复杂度校验，自定义模式须避免嵌套无界量词与歧义字符集回退（见 `register` 文档）。

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

- 可用空间不足判定：保留量为 50MB 与可用空间 10% 的较大者，可用空间不大于该保留量时判定空间不足并降级（检查带节流缓存，空间不足时不节流、恢复后立即重检）；
- 自动清理旧日志：大小清理（删除最旧文件直至低于 `max_total_size`）与年龄清理（删除早于 `retention_days` 的文件）独立执行——`max_total_size` 不可解析时过期清理仍然生效；两个分支都始终保留最新的 `keep_files` 个轮转文件；
- 等保场景：`encrypt=true` 且 `retention_days < 180` 时输出留存合规提示（等保 2.0 要求日志留存 ≥ 6 个月）。

**文件权限（审计加固）**：

- FileSink 活动日志文件以 `0600` 创建（此前仅 CLI 解密输出为 0600）；
- 轮转加密产物（`.enc`）与压缩产物（`.zst`/`.gz`）同样以 `0600` 创建；
- 新建的日志目录以 `0700` 设置权限（已存在目录不回改用户权限）。

**崩溃一致性（可选）**：

- `FileSinkConfig.fsync = true` 时每批写盘后执行 `sync_all`（默认关闭，吞吐优先）；
- 空闲期兜底刷盘：日志停止产生后，批量缓冲由轮转定时线程代为落盘，不再滞留内存。

### 数据库访问控制

**SQL 注入防护**：

- 表名白名单校验（`validate_table_name`）：仅允许字母、数字、下划线，必须以字母或下划线开头（`^[a-zA-Z_][a-zA-Z0-9_]*$`，无长度上限）；
- 业务数据一律经参数化查询写入，禁止字符串拼接；
- DDL（建表/分区）使用校验后的名称构建；
- **转义语义随实际后端**：`DbNexusAdapter` 的四个构造器（`with_full_config` / `from_pool` / `from_connection_pool`）用同一判据解析驱动——dbnexus 的 `DbConfig::database_type()`（与 dbnexus 建池选后端同源，scheme 大小写不敏感，`MYSQL://` 与 `mysql://` 同义）；URL scheme 无法判定或后端不受支持（Ladybug/Neo4j）时构造直接失败，不猜一个语义。建表方言与写入转义同源于该驱动，不会分叉。MySQL 默认把反斜杠当转义符，仅双写单引号仍可逃逸字符串字面量（反斜杠 + 单引号 → 闭合字面量 + 独立语句），按错误后端语义转义即成注入面；
- **DSN 凭据不外泄**：`DatabaseSinkConfig` 的 `Debug` 与 `Serialize` 均遮蔽 DSN 中的口令（`postgres://user:***@host/db`）——配置对象整体打印或整体回显（管理接口 JSON）不携带数据库凭据。遮蔽覆盖 userinfo、query/fragment 与 keyword/value 三种形态（凭据字段名 password/pwd/pass/token/secret/api_key/apikey/auth/key/sig/signature，大小写不敏感）；形态不可判定时不回显疑似口令原文。口令位是占位符（`***`）的 DSN 认证必然失败，配置校验显式拒绝该形态。

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
| **绑定约束（fail-closed）** | 绑定非回环地址（`0.0.0.0` / `::` / 内网 IP）且既无 `auth` 又无 `ip_whitelist` 时，配置校验返回 `Err`——`/metrics`、`/health` 不得无访问控制暴露 |
| **Host 校验（DNS rebinding）** | 对端地址为回环时，目标主机必须为回环主机（`localhost` / `127.0.0.1` / `[::1]`），否则 403——阻断恶意站点 rebind 到本机端口后跨源读取端点响应。目标主机取 `Host` 头，缺失时回退 URI authority（HTTP/2 只带 `:authority`），两者并存必须指向同一主机 |
| **响应面** | `/health` 只回显状态类别：sink 错误原文（含表名/约束名/路径）不进序列化输出，留在进程内与内部日志 |

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

# 4. 代码覆盖率（门禁 ≥ 95%）
cargo llvm-cov --features "sqlite http cli kit compression gzip parquet fast-masking" --lib --fail-under-lines 95

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
**维护者**: Kirky.X
**联系我们**: [Kirky-X@outlook.com](mailto:Kirky-X@outlook.com)

*本文档遵循 CC BY-SA 4.0 许可协议*
