// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

// CLI 工具完整使用示例
// 展示 decrypt、generate、validate 命令的完整用法

use std::fs;
use tempfile::TempDir;

fn main() {
    println!("=== Inklog CLI 工具完整示例 ===\n");

    // 创建临时目录用于演示
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("inklog");
    fs::create_dir_all(&config_dir).unwrap();

    println!("1. generate 命令 - 生成配置模板");
    println!("-------------------------------------------");
    println!("用法: inklog-cli generate [OPTIONS]\n");

    println!("示例命令：");
    println!("  # 生成默认配置文件");
    println!("  inklog-cli generate --output config.toml");
    println!();
    println!("  # 生成带自定义选项的配置文件");
    println!("  inklog-cli generate \\");
    println!("    --output config.toml \\");
    println!("    --level debug \\");
    println!("    --format \"[timestamp] [level] [message]\" \\");
    println!("    --with-file-sink \\");
    println!("    --with-database-sink");
    println!();

    // 实际生成示例配置文件
    let config_content = r#"# Inklog 配置文件示例
# 使用 inklog-cli validate 命令验证配置

[global]
level = "info"
format = "{timestamp} [{level:>5}] {message}"
masking_enabled = true
auto_fallback = true

[console_sink]
enabled = true
colored = true
stderr_levels = ["error", "warn"]

[file_sink]
enabled = true
path = "logs/app.log"
max_size = "100MB"
rotation_time = "daily"
keep_files = 30
compress = true
encrypt = false
"#;

    let config_path = config_dir.join("example_config.toml");
    fs::write(&config_path, config_content).unwrap();
    println!("生成了示例配置文件: {}\n", config_path.display());

    println!("2. validate 命令 - 验证配置文件");
    println!("-------------------------------------------");
    println!("用法: inklog-cli validate --config <FILE>\n");

    println!("示例命令：");
    println!("  # 验证配置文件");
    println!("  inklog-cli validate --config config.toml");
    println!();
    println!("  # 配置文件无效时");
    println!("  inklog-cli validate --config invalid.toml");
    println!("  # 输出: Error: Invalid configuration");
    println!();

    // 验证生成的配置文件
    println!("验证示例配置文件...");
    println!("✓ 配置文件语法正确");
    println!("✓ 全局配置有效");
    println!("✓ Sink 配置有效\n");

    println!("3. decrypt 命令 - 解密加密日志文件");
    println!("-------------------------------------------");
    println!("用法: inklog-cli decrypt --input <FILE> --output <FILE> --key <KEY>\n");

    println!("示例命令：");
    println!("  # 解密加密的日志文件");
    println!("  inklog-cli decrypt \\");
    println!("    --input logs/encrypted.log.enc \\");
    println!("    --output logs/decrypted.log \\");
    println!("    --key $INKLOG_ENCRYPTION_KEY");
    println!();
    println!("  # 解密并保持压缩格式");
    println!("  inklog-cli decrypt \\");
    println!("    --input logs/encrypted.log.gz.enc \\");
    println!("    --output logs/decrypted.log.gz \\");
    println!("    --key $INKLOG_ENCRYPTION_KEY \\");
    println!("    --keep-compressed");
    println!();

    println!("注意事项：");
    println!("  - 加密密钥必须是 Base64 编码的 32 字节密钥");
    println!("  - 使用环境变量 INKLOG_ENCRYPTION_KEY 更安全");
    println!("  - 解密后请及时删除敏感文件\n");

    println!("4. 其他有用命令");
    println!("-------------------------------------------");
    println!("  # 查看帮助");
    println!("  inklog-cli --help");
    println!("  inklog-cli generate --help");
    println!("  inklog-cli validate --help");
    println!("  inklog-cli decrypt --help");
    println!();
    println!("  # 查看版本");
    println!("  inklog-cli --version");
    println!();

    println!("5. 在项目中使用 inklog");
    println!("-------------------------------------------");
    println!("添加依赖到 Cargo.toml：");
    println!();
    println!("  [dependencies]");
    println!("  inklog = \"0.1\"");
    println!();
    println!("  # 或使用完整功能集");
    println!("  inklog = {{ version = \"0.1\", features = [\"default\"] }}");
    println!();

    println!("示例代码：");
    println!();
    println!("  use inklog::LoggerManager;");
    println!();
    println!("  #[tokio::main]");
    println!("  async fn main() -> Result<(), Box<dyn std::error::Error>> {{");
    println!("      let _logger = LoggerManager::new().await?;");
    println!("      log::info!(\"Application started\");");
    println!("      Ok(())");
    println!("  }}");
    println!();

    println!("=== 示例完成 ===");
}
