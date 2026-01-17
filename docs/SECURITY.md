# Inklog 安全指南

## 安全概览

Inklog 是一个**企业级安全优先**的 Rust 日志基础设施,专为需要严格数据保护和合规性要求的环境设计。我们的安全设计遵循以下核心原则:

### 安全设计原则

- **加密优先**: 默认支持 AES-256-GCM 军用级加密
- **零信任**: 所有敏感数据通过环境变量注入,从不硬编码
- **内存安全**: 使用 `zeroize` 确保敏感数据在内存中不留痕迹
- **深度防御**: 多层安全控制,从加密到访问控制
- **合规性设计**: 支持 GDPR、HIPAA、PCI-DSS 等合规要求

### 安全承诺

- ✅ **无硬编码密钥**: 所有密钥通过环境变量或密钥管理服务获取
- ✅ **内存安全**: 敏感数据自动清零,使用 Rust 内存安全保证
- ✅ **传输加密**: S3 传输使用 HTTPS/TLS,支持服务端加密
- ✅ **最小权限**: 数据库和文件访问遵循最小权限原则
- ✅ **审计追踪**: 可选的调试功能记录敏感操作
- ✅ **持续安全审计**: 使用 `cargo deny` 进行依赖安全检查

---

## 加密

Inklog 使用 **AES-256-GCM** (Galois/Counter Mode) 进行经过认证的加密,提供机密性和完整性保证。

### 加密架构

```rust
use aes_gcm::{Aes256Gcm, KeyInit, aead::{Aead, Nonce}};
use zeroize::Zeroizing;
use rand::Rng;
```

**加密流程**:

1. **密钥获取**: 从环境变量安全读取 32 字节密钥
2. **Nonce 生成**: 使用加密安全的随机数生成器创建 12 字节 nonce
3. **加密操作**: 使用 AES-256-GCM 加密数据
4. **完整性验证**: GCM 模式自动包含认证标签
5. **文件写入**: 按照版本化格式写入加密数据

### 密钥管理

#### 从环境变量安全获取密钥

```rust
fn get_encryption_key(env_var: &str) -> Result<[u8; 32], InklogError> {
    // 使用 Zeroizing 保护环境变量值
    let env_value = Zeroizing::new(std::env::var(env_var)?);
    
    // 支持 Base64 编码格式
    if let Ok(decoded) = general_purpose::STANDARD.decode(env_value.as_str()) {
        if decoded.len() == 32 {
            let mut result = [0u8; 32];
            result.copy_from_slice(&decoded);
            return Ok(result);
        }
    }
    
    // 支持原始 32 字节格式
    let raw_bytes = env_value.as_bytes();
    if raw_bytes.len() >= 32 {
        let mut result = [0u8; 32];
        result.copy_bytes[..32]);
_from_slice(&raw        return Ok(result);
    }
    
    Err(InklogError::ConfigError(
        "Key must be 32 bytes (256 bits)".into()
    ))
}
```

**密钥格式支持**:

| 格式 | 示例 | 说明 |
|------|------|------|
| **Base64** | `MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI=` | 推荐,易于管理 |
| **原始字节** | 32 字节二进制数据 | 需精确处理 |

**环境变量配置**:

```bash
# 设置加密密钥
export INKLOG_ENCRYPTION_KEY=$(openssl rand -base64 32)

# 或使用自定义密钥
export INKLOG_ENCRYPTION_KEY=your-32-byte-base64-encoded-key
```

### 加密/解密流程

#### 文件加密实现

```rust
fn encrypt_file(&self, path: &PathBuf) -> Result<PathBuf, InklogError> {
    let encrypted_path = path.with_extension("enc");
    
    // 1. 安全获取密钥
    let key = self.get_encryption_key()?;
    
    // 2. 生成随机 nonce (12 字节)
    let nonce: [u8; 12] = rand::thread_rng().gen();
    
    // 3. 读取明文
    let plaintext = std::fs::read_to_string(path)?;
    
    // 4. 使用 AES-256-GCM 加密
    let cipher = Aes256Gcm::new((&key).into());
    let nonce_slice = aes_gcm::Nonce::from_slice(&nonce);
    let ciphertext = cipher
        .encrypt(nonce_slice, plaintext.as_ref())
        .map_err(|e| InklogError::EncryptionError(e.to_string()))?;
    
    // 5. 写入加密文件
    let mut output_file = File::create(&encrypted_path)?;
    
    // 文件头: [8字节 MAGIC][2字节版本][2字节算法][12字节 nonce]
    output_file.write_all(MAGIC_HEADER)?;           // "ENCLOG1\0"
    output_file.write_all(&1u16.to_le_bytes())?;    // 版本 = 1
    output_file.write_all(&1u16.to_le_bytes())?;    // 算法 = 1 (AES-256-GCM)
    output_file.write_all(&nonce)?;                  // 12 字节 nonce
    output_file.write_all(&ciphertext)?;             // 加密数据
    
    // 6. 设置安全权限
    let mut perms = metadata.permissions();
    perms.set_mode(0o600);  // 仅所有者可读写
    file.set_permissions(perms)?;
    
    Ok(encrypted_path)
}
```

#### 文件解密实现 (CLI 工具)

```bash
# 使用 decrypt 命令解密日志文件
inklog decrypt \
  --input logs/app.log.enc \
  --output logs/app.log \
  --key-env INKLOG_DECRYPT_KEY

# 批量解密
inklog decrypt \
  --input "logs/*.enc" \
  --output decrypted/ \
  --batch \
  --key-env INKLOG_DECRYPT_KEY
```

**解密验证**:
- 自动检测文件格式 (V1 / Legacy)
- 验证 MAGIC_HEADER (`ENCLOG1\0`)
- 检查算法标识 (仅支持 AES-256-GCM)
- GCM 认证标签自动验证数据完整性

### 加密文件格式

#### V1 格式 (当前版本)

```
偏移    大小    描述
------  ------  ------------------
0-7     8字节   MAGIC: "ENCLOG1\0"
8-9     2字节   版本号: 1 (u16 little-endian)
10-11   2字节   算法标识: 1 (AES-256-GCM)
12-23   12字节  Nonce (随机数)
24+     N字节   密文 (包含 GCM 认证标签)
```

#### Legacy 格式 (向后兼容)

```
偏移    大小    描述
------  ------  ------------------
0-7     8字节   MAGIC: "ENCLOG1\0"
8-9     2字节   版本号: 1
10-21   12字节  Nonce (随机数)
22+     N字节   密文
```

**使用示例**:

```rust
use inklog::{FileSinkConfig, InklogConfig, LoggerManager};

// 配置加密日志
std::env::set_var("INKLOG_ENCRYPTION_KEY", "base64-encoded-32-byte-key");

let config = InklogConfig {
    file_sink: Some(FileSinkConfig {
        enabled: true,
        path: "logs/secure.log".into(),
        encrypt: true,
        encryption_key_env: Some("INKLOG_ENCRYPTION_KEY".into()),
        compress: false,  // 加密和压缩互斥
        ..Default::default()
    }),
    global: GlobalConfig {
        masking_enabled: true,  // 同时启用数据脱敏
        ..Default::default()
    },
    ..Default::default()
};

let logger = LoggerManager::with_config(config).await?;

// 日志将自动加密并脱敏
log::info!("User email: user@example.com");
// 输出: logs/secure.log.enc (加密 + 脱敏后)
```

---

## 数据脱敏

Inklog 内置全面的 **PII (个人身份信息)** 检测和脱敏功能,保护敏感数据不被记录到日志中。

### 内置 PII 检测模式

#### 1. 邮箱地址检测

**正则表达式**:
```rust
r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}"
```

**脱敏规则**: `user@example.com` → `***@***.***`

**代码示例**:
```rust
impl MaskRule {
    fn new_email_rule() -> Self {
        Self {
            pattern: LazyLock::new(|| {
                Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap()
            }),
            replacement: "***@***.***".to_string(),
        }
    }
}
```

#### 2. 电话号码检测 (中国手机号)

**正则表达式**:
```rust
r"\b1[3-9]\d{9}\b"
```

**脱敏规则**: `13812345678` → `138****5678`

#### 3. 身份证号检测 (中国 SSN)

**正则表达式**:
```rust
r"^(\d{6})(\d{8})(\d{3}[\dX])$"
```

**脱敏规则**: `110101199001011234` → `******1234` (保留后4位)

#### 4. 银行卡号检测

**正则表达式**:
```rust
r"(\d{4})(\d+)(\d{4})"
```

**脱敏规则**: `6222021234567890123` → `****-****-****-0123`

#### 5. API 密钥检测

**正则表达式**:
```rust
r"(?i)(api[_-]?key[^\s:=]*\s*[=:]\s*[a-zA-Z0-9_-]{20,})"
```

**脱敏规则**: `api_key=abcdefghij1234567890` → `api_key=***REDACTED***`

#### 6. AWS 凭据检测

**正则表达式**:
```rust
r"(?i)(AKIA|ABIA|ACCA|ASIA)[0-9A-Z]{16}"
```

**脱敏规则**: `AKIAIOSFODNN7EXAMPLE` → `***REDACTED***`

#### 7. JWT Token 检测

**正则表达式**:
```rust
r"(?i)eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*"
```

**脱敏规则**: 完整替换为 `***REDACTED_JWT***`

#### 8. 通用密钥检测

**正则表达式**:
```rust
r"(?i)([^\s:=]*(?:token|secret|key|password|passwd|pwd|credential)s?[^\s:=]*\s*[=:]\s*)([a-zA-Z0-9_\-\+]{16,})"
```

**脱敏规则**: 敏感字段值替换为 `***REDACTED***`

### 字段级别敏感检测

#### 敏感字段名称列表 (29 种模式)

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

#### 快速检测函数

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

### 自定义模式支持

**当前状态**: 当前版本使用硬编码的内置规则,暂不支持运行时自定义正则模式。

**自定义扩展建议**:

如需添加自定义 PII 模式,可以修改 `src/masking.rs` 中的 `DataMasker::new()` 方法:

```rust
impl DataMasker {
    pub fn new() -> Self {
        let rules = vec![
            // 内置规则
            MaskRule::new_email_rule(),
            MaskRule::new_phone_rule(),
            
            // 添加自定义规则
            MaskRule {
                pattern: LazyLock::new(|| {
                    Regex::new(r"CUSTOM_PATTERN").unwrap()
                }),
                replacement: "***CUSTOM***".to_string(),
            },
        ];
        Self { rules }
    }
}
```

### 脱敏集成

#### 自动脱敏流程

```rust
impl LogRecord {
    pub fn from_event(event: &Event) -> Self {
        let mut record = /* ... */;
        
        // 创建日志记录时自动应用脱敏
        if config.global.masking_enabled {
            record.mask_sensitive_fields();
        }
        
        record
    }
    
    fn mask_sensitive_fields(&mut self) {
        let masker = DataMasker::new();
        
        // 1. 脱敏日志消息 (正则模式)
        self.message = masker.mask(&self.message);
        
        // 2. 递归脱敏所有字段值
        for (_, v) in self.fields.iter_mut() {
            masker.mask_value(v);
        }
        
        // 3. 敏感字段完全脱敏 (字段名检测)
        for (k, v) in self.fields.iter_mut() {
            if Self::is_sensitive_key(k) {
                *v = Value::String("***MASKED***".to_string());
            }
        }
    }
}
```

#### 配置启用

```toml
# inklog_config.toml
[global]
masking_enabled = true  # 启用数据脱敏 (默认: true)
```

**环境变量**:

```bash
export INKLOG_MASKING_ENABLED=true
```

### 使用示例

#### 基础使用

```rust
use inklog::{InklogConfig, config::GlobalConfig, LoggerManager};

let config = InklogConfig {
    global: GlobalConfig {
        masking_enabled: true,  // 启用脱敏
        ..Default::default()
    },
    ..Default::default()
};

let logger = LoggerManager::with_config(config).await?;

// 自动脱敏
log::info!("User login: user@example.com, phone: 13812345678");
// 输出: User login: ***@***.***, phone: 138****5678

log::info!("API key: AKIAIOSFODNN7EXAMPLE");
// 输出: API key: ***REDACTED***

log::info!("Password: secret123");
// 输出: Password: ***MASKED***
```

#### 直接 API 使用

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
// 结果: {
//   "email": "***@***.***",
//   "phone": "138****5678",
//   "password": "***MASKED***",
//   "contacts": ["***@***.***", "138****2222"]
// }
```

---

## 内存安全

Inklog 使用 Rust 的内存安全保证和 `zeroize` 库,确保敏感数据在内存中不留痕迹。

### Zeroize 集成

#### SecretString 安全类型

```rust
use zeroize::{Zeroize, Zeroizing};

/// 敏感字符串包装器
/// - 在 Drop 时自动清零
/// - 序列化时跳过敏感数据
#[derive(Debug, Clone, Default)]
pub struct SecretString(Option<Zeroizing<String>>);

impl SecretString {
    pub fn new(value: String) -> Self {
        Self(Some(Zeroizing::new(value)))
    }
    
    /// 安全获取字符串引用 (不消耗值)
    pub fn as_str_safe(&self) -> Option<&str> {
        self.0.as_deref()
    }
    
    /// 带审计日志的安全获取 (仅调试模式)
    pub fn take_audited(&mut self, event: &str) -> Option<String> {
        #[cfg(feature = "debug")]
        tracing::debug!(
            event = event,
            "Sensitive data accessed via SecretString::take_audited()"
        );
        self.0.take()
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        if let Some(s) = &mut self.0 {
            s.zeroize();  // 自动清零内存
        }
    }
}

// 序列化时跳过敏感值
impl Serialize for SecretString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_none()  // 不序列化敏感数据
    }
}
```

#### 加密密钥的 Zeroize 使用

```rust
fn get_encryption_key(env_var: &str) -> Result<[u8; 32], InklogError> {
    // 使用 Zeroizing 包装环境变量值
    let env_value = Zeroizing::new(std::env::var(env_var)?);
    
    // ... 处理密钥 ...
    
    // env_value 超出作用域时自动清零
}
```

### 内存清理机制

**自动清理触发时机**:
1. **Drop 实现**: `SecretString`、`Zeroizing` 在作用域结束时自动清零
2. **手动清理**: 调用 `.zeroize()` 方法立即清理

**清理保证**:
- 确定性地覆盖内存 (不仅是释放)
- 编译器优化不会跳过清理操作 (使用 `volatile_write`)
- 适用于 Rust 的所有内存模型

### 无 Unsafe 代码

**安全性验证**:
```bash
# 检查项目中的 unsafe 代码块
grep -r "unsafe" src/
# 输出: No matches found

✅ Inklog 库核心代码不使用任何 unsafe 块
```

**安全优势**:
- 没有 C 风格指针操作
- 没有手动内存管理
- 编译器保证内存安全
- 没有缓冲区溢出风险

### 使用示例

#### 敏感凭据保护

```rust
use inklog::archive::SecretString;

// 安全存储 AWS 凭据
let config = S3ArchiveConfig {
    access_key_id: SecretString::new("AKIAIOSFODNN7EXAMPLE".to_string()),
    secret_access_key: SecretString::new("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
    ..Default::default()
};

// 安全使用
if let Some(key_id) = config.access_key_id.as_str_safe() {
    println!("Access Key ID: {}", key_id);
}

// config 超出作用域时,所有密钥自动清零
```

#### 加密密钥保护

```rust
use zeroize::Zeroizing;

fn encrypt_sensitive_data(data: &[u8]) -> Result<Vec<u8>> {
    // 从环境变量安全读取密钥
    let key_bytes = Zeroizing::new(
        std::env::var("ENCRYPTION_KEY")?
    );
    
    // 使用密钥加密
    let encrypted = /* ... */;
    
    // key_bytes 超出作用域时自动清零
    Ok(encrypted)
}
```

---

## 访问控制

Inklog 在文件、数据库和网络层面实施严格的访问控制,保护日志数据的机密性和完整性。

### 文件权限控制

#### Unix 文件权限设置

```rust
// src/sink/file.rs:284-287
let mut perms = metadata.permissions();
perms.set_mode(0o600);  // rw------- (仅所有者可读写)
if let Err(e) = file.set_permissions(perms) {
    eprintln!("Failed to set file permissions: {}", e);
}
```

**权限说明**:
- **0o600**: 所有者可读写,组和其他用户无权限
- **适用**: 所有加密日志文件、敏感配置文件
- **安全保证**: 防止其他用户读取加密日志

#### 文件创建安全

```rust
fn create_log_file(path: &PathBuf) -> Result<File, InklogError> {
    // 1. 验证路径不在系统目录
    validate_path_not_in_system_dirs(path)?;
    
    // 2. 确保父目录存在且安全
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
        set_secure_permissions(parent)?;
    }
    
    // 3. 创建文件
    let file = File::options()
        .write(true)
        .create_new(true)  // 防止覆盖现有文件
        .open(path)?;
    
    // 4. 设置安全权限
    let mut perms = file.metadata()?.permissions();
    perms.set_mode(0o600);
    file.set_permissions(perms)?;
    
    Ok(file)
}
```

### 数据库访问控制

#### 连接池管理

```rust
// src/sink/database.rs:267-271
let mut opt = ConnectOptions::new(url);
opt.max_connections(pool_size)      // 最大连接数限制
   .min_connections(2)            // 最小连接数
   .connect_timeout(Duration::from_secs(5))  // 连接超时
   .idle_timeout(Duration::from_secs(8));    // 空闲超时

Database::connect(opt).await
```

**安全特性**:
- **连接数限制**: 防止资源耗尽攻击
- **超时保护**: 避免长时间挂起的连接
- **连接复用**: 减少认证频繁

#### SQL 注入防护

**表名验证** (`src/sink/database.rs:141-173`):

```rust
/// 验证表名是否安全（防止 SQL 注入）
/// 只允许字母、数字、下划线,且必须以字母或下划线开头
fn validate_table_name(name: &str) -> Result<String, InklogError> {
    if name.is_empty() {
        return Err(InklogError::DatabaseError(
            "Table name cannot be empty".to_string(),
        ));
    }
    
    if name.len() > 128 {
        return Err(InklogError::DatabaseError(
            "Table name too long".to_string(),
        ));
    }
    
    // 验证首字符
    let first_char = name.chars().next()
        .ok_or_else(|| InklogError::DatabaseError("Empty name".to_string()))?;
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return Err(InklogError::DatabaseError(
            "Table name must start with letter or underscore".to_string()
        ));
    }
    
    // 验证所有字符
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(InklogError::DatabaseError(
            "Table name contains invalid characters".to_string()
        ));
    }
    
    Ok(name.to_string())
}
```

**分区名验证** (`src/sink/database.rs:175-213`):

```rust
/// 验证分区名称格式 (必须是 logs_YYYY_MM 格式)
fn validate_partition_name(partition_name: &str) -> Result<String, InklogError> {
    if !partition_name.starts_with("logs_") {
        return Err(InklogError::DatabaseError(
            "Partition name must start with 'logs_'".to_string()
        ));
    }
    
    // 验证日期部分格式 YYYY_MM
    let date_part = &partition_name[5..];
    if date_part.len() != 7 || date_part.chars().nth(4) != Some('_') {
        return Err(InklogError::DatabaseError(
            "Invalid partition date format, expected YYYY_MM".to_string()
        ));
    }
    
    let year = &date_part[..4];
    let month = &date_part[5..];
    
    // 验证年份和月份为有效数字
    if !year.chars().all(|c| c.is_ascii_digit()) {
        return Err(InklogError::DatabaseError("Invalid year".to_string()));
    }
    
    let month_num: u32 = month.parse().unwrap_or(0);
    if month_num == 0 || month_num > 12 {
        return Err(InklogError::DatabaseError("Invalid month".to_string()));
    }
    
    Ok(partition_name.to_string())
}
```

**使用验证后的名称** (`src/sink/database.rs:461-478`):

```rust
// 使用验证后的名称构建 SQL (避免 SQL 注入)
let validated_table = validate_table_name(&self.config.table_name)?;
let quoted_table = format!("\"{}\"", validated_table);  // PostgreSQL 引用
let quoted_partition = format!("\"{}\"", validated_partition);

let sql = format!(
    "CREATE TABLE IF NOT EXISTS {} PARTITION OF {} FOR VALUES FROM ('{}') TO ('{}')",
    quoted_table, quoted_partition, start_date, next_month
);

let stmt = Statement::from_string(db.get_database_backend(), sql);
db.execute_unprepared(&stmt.sql).await?;
```

#### 参数化查询

```rust
// Sea-ORM 自动使用参数化查询,防止 SQL 注入
Entity::insert_many(logs).exec(db).await

// 转换为: INSERT INTO logs (...) VALUES ($1, $2, $3), ...
```

### 最小权限原则

#### 数据库用户权限建议

**PostgreSQL**:
```sql
-- 创建专用日志用户
CREATE USER inklog_writer WITH PASSWORD 'secure_password';

-- 授予最小权限
GRANT CONNECT ON DATABASE logs TO inklog_writer;
GRANT USAGE ON SCHEMA public TO inklog_writer;
GRANT SELECT, INSERT ON ALL TABLES IN SCHEMA public TO inklog_writer;
GRANT SELECT, UPDATE ON ALL SEQUENCES IN SCHEMA public TO inklog_writer;

-- 授予分区创建权限 (如果需要)
GRANT CREATE ON SCHEMA public TO inklog_writer;
```

**MySQL**:
```sql
-- 创建专用日志用户
CREATE USER 'inklog_writer'@'%' IDENTIFIED BY 'secure_password';

-- 授予最小权限
GRANT INSERT, SELECT ON logs.* TO 'inklog_writer'@'%';
GRANT CREATE ON logs.* TO 'inklog_writer'@'%';  -- 分区创建
```

**SQLite**:
- 文件系统权限控制 (0o600)
- 仅本地访问

#### S3 访问控制

```rust
// 使用 IAM 角色而非访问密钥 (推荐)
let aws_config = aws_config::from_env()
    .region(Region::new(region.clone()))
    .load()
    .await;

// 或使用最小权限的访问密钥
let config = S3ArchiveConfig {
    bucket: "my-log-bucket".to_string(),
    region: "us-west-2".to_string(),
    access_key_id: SecretString::new("AKIA...".to_string()),
    secret_access_key: SecretString::new("...".to_string()),
    ..Default::default()
};
```

**IAM 策略示例**:
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:PutObject",
        "s3:GetObject",
        "s3:DeleteObject"
      ],
      "Resource": [
        "arn:aws:s3:::my-log-bucket/logs/*"
      ]
    }
  ]
}
```

### 路径遍历防护

**解密工具路径验证**:

```rust
// src/cli/decrypt.rs
fn validate_file_path(input_path: &Path, base_dir: &Path) -> Result<PathBuf> {
    // 规范化路径 (解析 ../ 和 ./)
    let canonical_input = std::fs::canonicalize(input_path)
        .map_err(|e| anyhow!("Failed to canonicalize input path: {}", e))?;
    let canonical_base = std::fs::canonicalize(base_dir)
        .map_err(|e| anyhow!("Failed to canonicalize base dir: {}", e))?;
    
    // 验证路径在基础目录内
    if !canonical_input.starts_with(&canonical_base) {
        return Err(anyhow!(
            "Path traversal detected: {} is outside of base directory {}",
            input_path.display(),
            base_dir.display()
        ));
    }
    
    Ok(canonical_input)
}
```

**防护保证**:
- 防止 `../../../etc/passwd` 等攻击
- 解析符号链接后的真实路径
- 限制在指定目录内操作

---

## 网络安全

Inklog 在 S3 归档和数据库通信中实施严格的网络安全措施。

### S3 通信安全

#### 默认 HTTPS 传输

```rust
// AWS SDK 默认使用 HTTPS
let aws_config = aws_config::from_env()
    .region(Region::new(region.clone()))
    .load()
    .await;

let client = aws_sdk_s3::Client::new(&aws_config);

// 所有请求自动使用 HTTPS
client.put_object()
    .bucket(bucket)
    .key(key)
    .body(data.into())
    .send()
    .await?;  // 通过 HTTPS/TLS 传输
```

**安全特性**:
- **TLS 1.2+**: 使用最新 TLS 版本
- **证书验证**: 自动验证 S3 服务器证书
- **加密传输**: 所有数据在传输中加密

#### S3 服务端加密 (SSE)

**支持的服务端加密类型**:

```rust
pub enum EncryptionAlgorithm {
    Aes256,        // SSE-S3: AWS 托管密钥
    AwsKms,        // SSE-KMS: KMS 托管密钥
    CustomerKey,    // SSE-C: 客户提供密钥 (待实现)
}

pub struct EncryptionConfig {
    pub algorithm: EncryptionAlgorithm,
    pub kms_key_id: Option<String>,      // KMS 密钥 ID
    pub customer_key: SecretString,       // 客户密钥 (SSE-C)
}
```

**配置 SSE-KMS 加密**:

```toml
# inklog_config.toml
[s3_archive]
enabled = true
bucket = "my-log-bucket"
region = "us-west-2"

[s3_archive.encryption]
algorithm = "AwsKms"
kms_key_id = "arn:aws:kms:us-west-2:123456789012:key/12345678-1234-1234-1234-123456789012"
```

**环境变量**:

```bash
export INKLOG_S3_ENCRYPTION_ALGORITHM=AWSKMS
export INKLOG_S3_ENCRYPTION_KMS_KEY_ID=arn:aws:kms:us-west-2:...
```

**KMS 加密优势**:
- **密钥轮换**: 自动密钥轮换支持
- **访问控制**: 细粒度的 KMS 权限管理
- **审计日志**: AWS CloudTrail 记录所有加密操作
- **合规性**: 满足 HIPAA、PCI-DSS 等要求

#### S3 访问日志

启用 S3 访问日志以监控所有访问:

```bash
# AWS CLI
aws s3api put-bucket-logging \
  --bucket my-log-bucket \
  --bucket-logging-status '{"LoggingEnabled":{"TargetBucket":"my-log-bucket","TargetPrefix":"access-logs/"}}'
```

### 数据库安全连接

#### 安全连接配置

**PostgreSQL SSL 模式**:

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

**MySQL SSL**:

```rust
let url = "mysql://user:pass@localhost/logs?ssl_mode=REQUIRED".to_string();
```

**SQLite**: 本地文件访问,文件权限控制

#### 连接超时和重试

```rust
let mut opt = ConnectOptions::new(url);
opt.connect_timeout(Duration::from_secs(5))   // 连接超时 5 秒
   .idle_timeout(Duration::from_secs(8))      // 空闲超时 8 秒
   .max_lifetime(Duration::from_secs(3600))   // 连接最大生存时间 1 小时
   .acquire_timeout(Duration::from_secs(30)); // 获取连接超时 30 秒
```

### 网络最佳实践

#### 1. 使用 VPC 端点 (AWS)

```rust
// 配置 VPC 端点访问 S3 (私有网络)
let config = aws_config::from_env()
    .region(Region::new(region.clone()))
    .endpoint_url("https://vpce-xxxxx-s3.us-west-2.vpce.amazonaws.com")  // VPC 端点
    .load()
    .await;
```

**优势**:
- 私有网络访问 S3
- 无需互联网网关
- 增强安全性

#### 2. IP 白名单

```json
// S3 存储桶策略
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Deny",
      "Principal": "*",
      "Action": "s3:*",
      "Resource": [
        "arn:aws:s3:::my-log-bucket/*"
      ],
      "Condition": {
        "NotIpAddress": {
          "aws:SourceIp": ["192.0.2.0/24", "203.0.113.0/24"]
        }
      }
    }
  ]
}
```

#### 3. 数据库防火墙规则

```bash
# PostgreSQL pg_hba.conf (仅允许应用服务器访问)
hostssl logs    inklog_writer    10.0.1.0/24    scram-sha-256
```

---

## 合规性

Inklog 的安全设计支持多种合规性要求,包括 GDPR、HIPAA 和 PCI-DSS。

### GDPR (通用数据保护条例)

#### 个人数据处理

**PII 数据检测和脱敏**:

```rust
// 自动脱敏 GDPR 敏感数据
log::info!("User registration: email=user@example.com, phone=13812345678");
// 输出: User registration: email=***@***.***, phone=138****5678
```

**支持的 GDPR 敏感数据类型**:
- ✅ 电子邮箱地址 (邮箱脱敏)
- ✅ 电话号码 (电话脱敏)
- ✅ 身份标识符 (身份证脱敏)
- ✅ 网络标识符 (IP 地址脱敏 - 自定义)

#### 数据主体权利

**被遗忘权**:

```rust
// 清理过期日志 (超过保留期限)
pub async fn cleanup_old_logs(
    db: &DatabaseConnection,
    cutoff_date: DateTime<Utc>,
) -> Result<u64, InklogError> {
    Entity::delete_many()
        .filter(Column::Timestamp.lt(cutoff_date))
        .exec(db)
        .await
        .map_err(|e| InklogError::DatabaseError(e.to_string()))?;
    
    Ok(rows_affected)
}
```

**数据访问和携带权**:

```rust
// 导出特定用户的日志
pub async fn export_user_logs(
    user_id: &str,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<Vec<LogRecord>, InklogError> {
    Entity::find()
        .filter(Column::Fields.contains(format!("\"user_id\":\"{}\"", user_id)))
        .filter(Column::Timestamp.gte(start_date))
        .filter(Column::Timestamp.lt(end_date))
        .all(db)
        .await
}
```

#### 数据保护措施

**加密存储** (GDPR 第 32 条):

- ✅ AES-256-GCM 静态数据加密
- ✅ HTTPS/TLS 传输加密
- ✅ 密钥管理服务集成 (KMS)

**数据最小化** (GDPR 第 5 条):

```rust
// 仅记录必要的日志级别
let config = InklogConfig {
    global: GlobalConfig {
        level: "warn".to_string(),  // 不记录 DEBUG/TRACE 信息
        masking_enabled: true,      // 自动脱敏 PII
        ..Default::default()
    },
    ..Default::default()
};
```

### HIPAA (健康保险流通与责任法案)

#### 受保护健康信息 (PHI) 保护

**敏感字段检测**:

```rust
// 扩展脱敏规则以包含 PHI 字段
const HIPAA_SENSITIVE_FIELDS: &[&str] = &[
    "patient_id", "medical_record", "diagnosis", "treatment",
    "medication", "ssn", "insurance_number"
];

// 日志中自动脱敏 PHI
log::info!("Patient admitted: patient_id=123456, diagnosis=condition_X");
// 输出: Patient admitted: patient_id=******, diagnosis=*******
```

#### 安全审计日志

**启用调试模式记录**:

```rust
// Cargo.toml
[dependencies]
inklog = { version = "0.1", features = ["debug"] }

// 记录敏感操作
#[cfg(feature = "debug")]
tracing::debug!(
    event = "data_export",
    user_id = user_id,
    records = count,
    "PHI data exported for audit"
);
```

**审计事件**:
- 🔍 敏感数据访问
- 🔍 密钥使用
- 🔍 日志导出操作
- 🔍 加密/解密操作

#### 技术保障措施

**物理和环境安全**:
- ✅ 云存储安全 (AWS S3)
- ✅ 网络隔离 (VPC 端点)

**访问控制**:
- ✅ 最小权限数据库用户
- ✅ S3 存储桶策略
- ✅ KMS 密钥访问控制

**传输安全**:
- ✅ TLS 1.2+ 数据库连接
- ✅ HTTPS S3 传输
- ✅ 服务端加密 (SSE-KMS)

### PCI-DSS (支付卡行业数据安全标准)

#### 支付卡数据保护

**信用卡号脱敏**:

```rust
// 自动脱敏信用卡号
log::info!("Payment processed: card_number=6222021234567890123");
// 输出: Payment processed: card_number=****-****-****-0123
```

**PCI-DSS 合规要求**:

| 要求 | Inklog 实现 | 状态 |
|------|------------|------|
| **禁止存储完整卡号** | 信用卡号自动脱敏 | ✅ |
| **加密传输** | HTTPS/TLS | ✅ |
| **加密存储** | AES-256-GCM | ✅ |
| **访问控制** | 文件权限 0o600 | ✅ |
| **日志监控** | 结构化日志 + 审计 | ✅ |
| **定期漏洞扫描** | `cargo deny check` | ✅ |

#### 支付日志最佳实践

**不记录的敏感信息**:
- ❌ CVV/CVC 码
- ❌ PIN 码
- ❌ 完整磁条数据
- ❌ 完整卡号 (仅记录后 4 位)

```rust
// ✅ 正确的做法
log::info!("Payment authorized: card_last4=0123, amount=100.00");

// ❌ 错误的做法
log::info!("Payment: card_number=6222021234567890123, cvv=123");
// 这将自动脱敏为: Payment: card_number=****-****-****-0123, cvv=***
```

### 合规性检查清单

#### GDPR 合规性

- [ ] 启用数据脱敏 (`INKLOG_MASKING_ENABLED=true`)
- [ ] 配置数据保留期限 (`retention_days`)
- [ ] 实施加密日志 (`encrypt=true`)
- [ ] 定期清理过期日志 (`cargo run --example cleanup`)
- [ ] 记录数据访问审计日志 (启用 `debug` feature)

#### HIPAA 合规性

- [ ] 使用 SSE-KMS 加密 (`INKLOG_S3_ENCRYPTION_ALGORITHM=AWSKMS`)
- [ ] 限制数据库访问权限 (专用用户)
- [ ] 启用审计日志 (features=["debug"])
- [ ] 实施 PHI 字段脱敏 (自定义规则)
- [ ] 配置网络隔离 (VPC 端点)

#### PCI-DSS 合规性

- [ ] 启用信用卡号脱敏 (内置规则)
- [ ] 不记录 CVV/CVC 码 (代码审查)
- [ ] 使用 TLS 1.2+ 数据库连接
- [ ] 定期运行安全扫描 (`cargo deny check`)
- [ ] 配置日志轮转和归档

---

## 安全最佳实践

### 1. 密钥管理

#### ✅ 推荐做法

**使用密钥管理服务 (KMS)**:

```rust
// 配置 KMS 加密 (推荐)
let config = S3ArchiveConfig {
    enabled: true,
    bucket: "my-log-bucket".to_string(),
    region: "us-west-2".to_string(),
    encryption: Some(EncryptionConfig {
        algorithm: EncryptionAlgorithm::AwsKms,
        kms_key_id: Some("arn:aws:kms:...".to_string()),
        customer_key: SecretString::new("".to_string()),
    }),
    ..Default::default()
};
```

**环境变量管理**:

```bash
# 使用 .env 文件 (不要提交到 git)
echo "INKLOG_ENCRYPTION_KEY=$(openssl rand -base64 32)" >> .env
echo ".env" >> .gitignore

# 加载环境变量
dotenv::dotenv().ok();
```

**定期密钥轮换**:

```rust
// 每 90 天轮换密钥
const KEY_ROTATION_DAYS: u64 = 90;

let key_created_at = get_key_creation_date()?;
if Utc::now().signed_duration_since(key_created_at).num_days() > KEY_ROTATION_DAYS as i64 {
    rotate_encryption_key()?;
}
```

#### ❌ 避免的做法

```rust
// ❌ 不要硬编码密钥
const ENCRYPTION_KEY: &[u8; 32] = &[0x01, 0x02, ...];

// ❌ 不要在日志中输出密钥
log::debug!("Encryption key: {:?}", key);

// ❌ 不要使用弱密钥
let weak_key = "password123";  // 容易被破解
```

### 2. 文件和目录安全

#### 安全文件权限

```rust
// 敏感文件设置为 0o600 (仅所有者可读写)
std::fs::set_permissions("logs/encrypted.log.enc", PermissionsExt::from_mode(0o600))?;

// 目录设置为 0o700 (仅所有者可访问)
std::fs::set_permissions("logs/", PermissionsExt::from_mode(0o700))?;
```

#### 安全目录结构

```
/var/log/inklog/
├── .env                    # 环境变量 (0o600)
├── config.toml            # 配置文件 (0o600)
└── app/
    ├── secure.log.enc       # 加密日志 (0o600)
    └── backup/
        └── 2026-01/
            └── app.log.enc  # 归档日志 (0o600)
```

**配置文件保护**:

```bash
# 限制配置文件权限
chmod 600 /etc/inklog/config.toml
chown inklog:inklog /etc/inklog/config.toml
```

### 3. 数据库安全

#### 使用专用数据库用户

```sql
-- PostgreSQL: 创建最小权限用户
CREATE USER inklog_writer WITH PASSWORD 'secure_random_password';

-- 仅授予必要的权限
GRANT CONNECT ON DATABASE logs TO inklog_writer;
GRANT USAGE ON SCHEMA public TO inklog_writer;
GRANT INSERT, SELECT ON logs TO inklog_writer;
```

#### 启用 SSL/TLS

```rust
// 强制使用 SSL 连接
let url = "postgres://user:pass@localhost/logs?sslmode=verify-full".to_string();

let opt = ConnectOptions::new(url);
opt.ssl_mode(ssl_mode::Require);
```

**SSL 证书验证**:

```rust
// 配置 CA 证书路径
let mut opt = ConnectOptions::new("postgres://...");
opt.ssl_options(ssl::SslConnector::builder()
    .ca_file("/path/to/ca.crt")
    .build()?);
```

### 4. S3 存储安全

#### 使用 IAM 角色 (推荐)

```rust
// 从实例元数据获取临时凭证 (无需硬编码)
let config = aws_config::from_env()
    .region(Region::new("us-west-2".to_string()))
    .load()
    .await;

let client = aws_sdk_s3::Client::new(&config);
```

**IAM 角色策略**:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:PutObject",
        "s3:GetObject",
        "s3:DeleteObject"
      ],
      "Resource": [
        "arn:aws:s3:::my-log-bucket/logs/*",
        "arn:aws:s3:::my-log-bucket/logs"
      ]
    }
  ]
}
```

#### 启用存储桶策略

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "DenyUnencryptedUploads",
      "Effect": "Deny",
      "Principal": "*",
      "Action": "s3:PutObject",
      "Resource": "arn:aws:s3:::my-log-bucket/*",
      "Condition": {
        "StringNotEquals": {
          "s3:x-amz-server-side-encryption": "AES256"
        }
      }
    },
    {
      "Sid": "DenyHTTP",
      "Effect": "Deny",
      "Principal": "*",
      "Action": "s3:*",
      "Resource": "arn:aws:s3:::my-log-bucket/*",
      "Condition": {
        "Bool": {
          "aws:SecureTransport": "false"
        }
      }
    }
  ]
}
```

### 5. 日志配置安全

#### 启用数据脱敏

```toml
[global]
masking_enabled = true  # 默认启用

[global.log_format]
# 自定义格式,避免记录敏感信息
format = "{timestamp} [{level}] {target} - {message}"
```

#### 限制日志级别

```rust
let config = InklogConfig {
    global: GlobalConfig {
        level: "info".to_string(),  // 不记录 DEBUG/TRACE
        ..Default::default()
    },
    ..Default::default()
};
```

**生产环境推荐**:
- `info`: 一般日志
- `warn`: 警告信息
- `error`: 错误信息

**调试环境**:
- `debug`: 调试信息
- `trace`: 详细追踪信息

### 6. 依赖安全

#### 定期安全审计

```bash
# 检查已知漏洞
cargo deny check advisories

# 检查许可协议
cargo deny check bans

# 检查来源可靠性
cargo deny check sources
```

#### 锁定依赖版本

```toml
# Cargo.toml
[dependencies]
inklog = "0.1"  # 使用 ^0.1.x 范围
```

```bash
# 生成 Cargo.lock (锁定确切版本)
cargo build
git add Cargo.lock  # 提交到版本控制
```

### 7. 监控和审计

#### 启用健康检查

```rust
use inklog::LoggerManager;

let logger = LoggerManager::with_config(config).await?;

// 定期检查日志系统健康状态
tokio::spawn(async move {
    loop {
        let health = logger.get_health_status();
        if !health.sinks[&"file"].is_healthy {
            eprintln!("WARNING: File sink is unhealthy!");
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
});
```

#### 审计敏感操作

```rust
// 启用 debug feature
#[cfg(feature = "debug")]
{
    tracing::debug!(
        event = "encryption_key_used",
        key_env = "INKLOG_ENCRYPTION_KEY",
        timestamp = Utc::now(),
        "Encryption operation performed"
    );
}
```

### 8. 故障处理

#### 优雅降级

```rust
// 数据库故障时回退到文件
pub fn fallback_to_file(&mut self) -> Result<(), InklogError> {
    if let Some(sink) = &mut self.fallback_sink {
        for record in &self.buffer {
            let _ = sink.write(record);
        }
    }
    Ok(())
}
```

#### 错误记录

```rust
// 不记录敏感信息到错误消息
log::error!(
    "Failed to decrypt file: {}. Error: {}",
    file_path.display(),  // ✅ 记录文件路径
    // ❌ 不要记录密钥或敏感数据
);
```

---

## 报告安全问题

如果您发现 Inklog 的安全漏洞,请负责任地向我们报告。

### 安全报告流程

#### 1. 不要公开披露

**❌ 避免**:
- ❌ 在 GitHub Issues 中公开安全漏洞
- ❌ 在社交媒体上披露漏洞细节
- ❌ 在未经授权的情况下进行漏洞利用

**✅ 推荐**:
- ✅ 通过私密渠道报告漏洞
- ✅ 提供复现步骤和影响评估
- ✅ 给予足够的时间修复

#### 2. 报告方式

**首选方式**:
- 邮件: `security@inklog.dev`
- PGP 密钥: (将在安全页面提供)

**备选方式**:
- GitHub Security Advisories: https://github.com/Kirky-X/inklog/security/advisories

#### 3. 报告内容

请包含以下信息:

**漏洞描述**:
- 漏洞类型 (注入、XSS、认证绕过等)
- 影响范围 (哪些版本受影响)
- 严重程度 (CVSS 评分)

**复现步骤**:
- 详细的重现步骤
- 代码示例或配置
- 预期行为 vs 实际行为

**影响评估**:
- 数据泄露可能性
- 系统可用性影响
- 业务影响范围

**缓解措施**:
- 临时缓解方案
- 建议的修复方向

#### 4. 响应时间表

| 阶段 | 时间 | 行动 |
|------|------|------|
| **确认收到** | 24 小时内 | 确认收到安全报告 |
| **初步评估** | 48 小时内 | 评估漏洞严重性 |
| **修复开发** | 7-14 天 | 开发并测试修复 |
| **补丁发布** | 修复完成后 | 发布安全补丁 |
| **公开披露** | 修复后 7-30 天 | 发布安全公告 (协调披露) |

#### 5. 协调披露流程

```
[Day 0]  研究者报告漏洞
         ↓
[Day 1]   Inklog 确认并评估
         ↓
[Day 7]   修复开发完成
         ↓
[Day 10]  补丁发布到私有预览
         ↓
[Day 14]  公开发布 + 安全公告
```

**影响因素**:
- 漏洞严重程度 (严重漏洞优先处理)
- 修复复杂度
- 已知的公开利用情况

### 安全赏金计划

#### 奖励范围

**符合条件的安全漏洞**:
- 🔴 **严重**: RCE、SQL 注入、认证绕过 - $1000
- 🟠 **高危**: 敏感数据泄露、XSS - $500
- 🟡 **中等**: CSRF、信息泄露 - $250
- 🟢 **低**: 轻微安全问题 - $100

#### 奖励标准

| 严重性 | CVSS 评分 | 奖励金额 |
|--------|-----------|---------|
| **严重** | 9.0 - 10.0 | $1000 |
| **高危** | 7.0 - 8.9 | $500 |
| **中等** | 4.0 - 6.9 | $250 |
| **低** | 0.1 - 3.9 | $100 |

#### 排除范围

**不符合条件的问题**:
- ❌ 已知漏洞的重复报告
- ❌ 需要物理访问的漏洞
- ❌ 社会工程攻击
- ❌ 第三方依赖的漏洞
- ❌ 最佳实践违规 (非安全漏洞)

### 已知安全问题

查看当前已知安全问题:

- **GitHub Security Advisories**: https://github.com/Kirky-X/inklog/security/advisories
- **更新日志**: CHANGELOG.md

### 安全更新订阅

订阅安全更新:

1. **Watch GitHub Repository**:
   - 访问 https://github.com/Kirky-X/inklog
   - 点击 "Watch" → "Custom"
   - 勾选 "Releases" 和 "Security alerts"

2. **订阅邮件列表**:
   - (待提供)

3. **RSS 订阅**:
   - (待提供)

### 安全资源

**学习资源**:
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [CWE Top 25](https://cwe.mitre.org/top25/)
- [Rust Security Guidelines](https://doc.rust-lang.org/nomicon/unsafe.html)

**工具推荐**:
- `cargo-audit`: 依赖漏洞扫描
- `cargo-deny`: 许可证和来源检查
- `grype`: 容器镜像漏洞扫描

**社区**:
- Rust 安全工作组: https://github.com/rustsec/advisory-db

---

## 附录

### A. 配置示例

#### 完整安全配置 (inklog_config.toml)

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

[s3_archive]
enabled = true
bucket = "my-log-bucket"
region = "us-west-2"
access_key_id_env = "INKLOG_S3_ACCESS_KEY_ID"  # 通过 SecretString 保护
secret_access_key_env = "INKLOG_S3_SECRET_ACCESS_KEY"
archive_interval_days = 7
local_retention_days = 30
prefix = "logs/"
compression = "Zstd"
storage_class = "Glacier"

[s3_archive.encryption]
algorithm = "AwsKms"
kms_key_id = "arn:aws:kms:us-west-2:123456789012:key/12345678-1234-1234-1234-123456789012"
```

#### 环境变量示例 (.env)

```bash
# 全局配置
INKLOG_LEVEL=info
INKLOG_MASKING_ENABLED=true

# 文件加密
INKLOG_ENCRYPTION_KEY=$(openssl rand -base64 32)

# 数据库连接
INKLOG_DB_DRIVER=postgres
INKLOG_DB_URL=postgres://inklog_writer:${DB_PASSWORD}@localhost/logs?sslmode=require
INKLOG_DB_POOL_SIZE=10

# S3 凭据
INKLOG_S3_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
INKLOG_S3_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
INKLOG_S3_ENCRYPTION_ALGORITHM=AWSKMS
INKLOG_S3_ENCRYPTION_KMS_KEY_ID=arn:aws:kms:us-west-2:123456789012:key/12345678-1234-1234-1234-123456789012

# 解密密钥
INKLOG_DECRYPT_KEY=$INKLOG_ENCRYPTION_KEY
```

### B. CLI 命令速查

```bash
# 生成配置模板
inklog generate --config-type full --output ./config/

# 生成环境变量示例
inklog generate --env-example --output ./

# 验证配置
inklog validate --config ./config/inklog_config.toml

# 检查系统先决条件
inklog validate --prerequisites

# 解密日志文件
inklog decrypt \
  --input logs/encrypted.log.enc \
  --output logs/decrypted.log \
  --key-env INKLOG_DECRYPT_KEY

# 批量解密目录
inklog decrypt \
  --input logs/*.enc \
  --output decrypted/ \
  --batch \
  --key-env INKLOG_DECRYPT_KEY

# 递归解密目录
inklog decrypt \
  --input logs/ \
  --output decrypted/ \
  --recursive \
  --key-env INKLOG_DECRYPT_KEY
```

### C. 安全检查命令

```bash
# 1. 运行所有测试
cargo test --all-features

# 2. 运行 Clippy (检查代码质量)
cargo clippy --all-targets --all-features -- -D warnings

# 3. 格式检查
cargo fmt --all -- --check

# 4. 安全审计
cargo deny check advisories
cargo deny check bans
cargo deny check licenses

# 5. 代码覆盖率
cargo tarpaulin --out Html --all-features

# 6. 检查文件权限
find logs/ -type f -exec chmod 600 {} \;
find logs/ -type d -exec chmod 700 {} \;

# 7. 验证加密密钥
openssl rand -base64 32 > /dev/null  # 生成测试密钥
```

### D. 术语表

| 术语 | 定义 |
|------|------|
| **AES-256-GCM** | 高级加密标准 256 位,使用伽罗瓦/计数器模式 |
| **Nonce** | 密码学随机数,用于加密过程中的唯一性 |
| **Zeroize** | 安全清零内存的技术 |
| **PII** | 个人身份信息 (Personally Identifiable Information) |
| **PHI** | 受保护健康信息 (Protected Health Information) |
| **KMS** | 密钥管理服务 (Key Management Service) |
| **SSE** | 服务端加密 (Server-Side Encryption) |
| **TLS** | 传输层安全协议 (Transport Layer Security) |
| **GDPR** | 通用数据保护条例 (General Data Protection Regulation) |
| **HIPAA** | 健康保险流通与责任法案 (Health Insurance Portability and Accountability Act) |
| **PCI-DSS** | 支付卡行业数据安全标准 (Payment Card Industry Data Security Standard) |

### E. 故障排除

#### 常见安全问题

**问题 1**: 解密失败 "Authentication failed"

**原因**: 密钥不匹配或文件损坏

**解决方案**:
```bash
# 验证环境变量
echo $INKLOG_DECRYPT_KEY | openssl enc -base64 -d | xxd

# 重新生成密钥
export INKLOG_DECRYPTION_KEY=$(openssl rand -base64 32)
```

**问题 2**: 文件权限错误 "Permission denied"

**解决方案**:
```bash
# 修复文件权限
chmod 600 logs/*.enc
chown $(whoami):$(whoami) logs/*.enc
```

**问题 3**: S3 上传失败 "Access Denied"

**原因**: IAM 权限不足

**解决方案**:
```bash
# 验证 IAM 权限
aws iam get-user-policy --user-name inklog-writer --policy-name InklogS3Access

# 或使用临时凭证 (推荐)
aws sts assume-role --role-arn arn:aws:iam::123456789012:role/InklogRole
```

---

**文档版本**: 1.0  
**最后更新**: 2026-01-17  
**维护者**: Inklog Security Team  

**联系我们**: security@inklog.dev

---

*本文档遵循 CC BY-SA 4.0 许可协议*
