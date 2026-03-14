// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

// CLI 工具集成测试
// 测试 decrypt、generate、validate 命令的完整功能

#[cfg(all(test, feature = "cli"))]
mod cli_test {
    use assert_cmd::Command;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // === Generate 命令测试 ===

    #[tokio::test]
    async fn test_generate_default_config() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("config.toml");
        
        let mut cmd = Command::cargo_bin("inklog-cli").unwrap();
        cmd.arg("generate")
            .arg("--output")
            .arg(&output_path)
            .assert()
            .success();
        
        assert!(output_path.exists());
        
        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("[global]"));
        assert!(content.contains("[console_sink]"));
        assert!(content.contains("[file_sink]"));
    }

    #[tokio::test]
    async fn test_generate_with_options() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("custom_config.toml");
        
        let mut cmd = Command::cargo_bin("inklog-cli").unwrap();
        cmd.arg("generate")
            .arg("--output")
            .arg(&output_path)
            .arg("--level")
            .arg("debug")
            .arg("--format")
            .arg("custom")
            .assert()
            .success();
        
        assert!(output_path.exists());
        
        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("level = \"debug\""));
    }

    // === Validate 命令测试 ===

    #[tokio::test]
    async fn test_validate_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("valid_config.toml");
        
        let valid_config = r#"
[global]
level = "info"
format = "{timestamp} [{level}] {message}"

[console_sink]
enabled = true
colored = true
"#;
        
        let mut file = File::create(&config_path).unwrap();
        file.write_all(valid_config.as_bytes()).unwrap();
        
        let mut cmd = Command::cargo_bin("inklog-cli").unwrap();
        cmd.arg("validate")
            .arg("--config")
            .arg(&config_path)
            .assert()
            .success()
            .stdout(predicates::str::contains("valid"));
    }

    #[tokio::test]
    async fn test_validate_invalid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid_config.toml");
        
        let invalid_config = r#"
[global]
level = "invalid_level"
format = ""
"#;
        
        let mut file = File::create(&config_path).unwrap();
        file.write_all(invalid_config.as_bytes()).unwrap();
        
        let mut cmd = Command::cargo_bin("inklog-cli").unwrap();
        cmd.arg("validate")
            .arg("--config")
            .arg(&config_path)
            .assert()
            .failure();
    }

    #[tokio::test]
    async fn test_validate_nonexistent_config() {
        let mut cmd = Command::cargo_bin("inklog-cli").unwrap();
        cmd.arg("validate")
            .arg("--config")
            .arg("/nonexistent/config.toml")
            .assert()
            .failure();
    }

    // === Decrypt 命令测试 ===

    #[tokio::test]
    async fn test_decrypt_valid_encrypted_file() {
        let temp_dir = TempDir::new().unwrap();
        let encrypted_path = temp_dir.path().join("encrypted.log.enc");
        let output_path = temp_dir.path().join("decrypted.log");
        
        // 创建测试加密文件
        // 注意：这里需要使用有效的加密文件格式
        let test_content = b"Test log entry\n";
        fs::write(&encrypted_path, test_content).unwrap();
        
        let key = "MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI=";
        
        let mut cmd = Command::cargo_bin("inklog-cli").unwrap();
        cmd.arg("decrypt")
            .arg("--input")
            .arg(&encrypted_path)
            .arg("--output")
            .arg(&output_path)
            .arg("--key")
            .arg(key)
            .assert()
            .success();
        
        // 验证输出文件存在
        assert!(output_path.exists());
    }

    #[tokio::test]
    async fn test_decrypt_with_invalid_key() {
        let temp_dir = TempDir::new().unwrap();
        let encrypted_path = temp_dir.path().join("encrypted.log.enc");
        let output_path = temp_dir.path().join("decrypted.log");
        
        // 创建测试文件
        fs::write(&encrypted_path, b"some encrypted content").unwrap();
        
        let mut cmd = Command::cargo_bin("inklog-cli").unwrap();
        cmd.arg("decrypt")
            .arg("--input")
            .arg(&encrypted_path)
            .arg("--output")
            .arg(&output_path)
            .arg("--key")
            .arg("invalid_key_base64encoded")
            .assert()
            .failure();
    }

    // === CLI 帮助测试 ===

    #[tokio::test]
    async fn test_cli_help() {
        let mut cmd = Command::cargo_bin("inklog-cli").unwrap();
        cmd.arg("--help")
            .assert()
            .success()
            .stdout(predicates::str::contains("Inklog CLI"));
    }

    #[tokio::test]
    async fn test_generate_help() {
        let mut cmd = Command::cargo_bin("inklog-cli").unwrap();
        cmd.arg("generate")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicates::str::contains("Generate"));
    }

    #[tokio::test]
    async fn test_validate_help() {
        let mut cmd = Command::cargo_bin("inklog-cli").unwrap();
        cmd.arg("validate")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicates::str::contains("Validate"));
    }

    #[tokio::test]
    async fn test_decrypt_help() {
        let mut cmd = Command::cargo_bin("inklog-cli").unwrap();
        cmd.arg("decrypt")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicates::str::contains("Decrypt"));
    }

    // === CLI 版本测试 ===

    #[tokio::test]
    async fn test_cli_version() {
        let mut cmd = Command::cargo_bin("inklog-cli").unwrap();
        cmd.arg("--version")
            .assert()
            .success()
            .stdout(predicates::str::contains("inklog-cli"));
    }
}

#[cfg(not(feature = "cli"))]
mod cli_test {
    // 当 cli 特性未启用时，跳过这些测试
    #[tokio::test]
    async fn test_generate_default_config() {
        println!("Skipping test: requires --features \"cli\"");
    }
    
    #[tokio::test]
    async fn test_generate_with_options() {
        println!("Skipping test: requires --features \"cli\"");
    }
    
    #[tokio::test]
    async fn test_validate_valid_config() {
        println!("Skipping test: requires --features \"cli\"");
    }
    
    #[tokio::test]
    async fn test_validate_invalid_config() {
        println!("Skipping test: requires --features \"cli\"");
    }
    
    #[tokio::test]
    async fn test_validate_nonexistent_config() {
        println!("Skipping test: requires --features \"cli\"");
    }
    
    #[tokio::test]
    async fn test_decrypt_valid_encrypted_file() {
        println!("Skipping test: requires --features \"cli\"");
    }
    
    #[tokio::test]
    async fn test_decrypt_with_invalid_key() {
        println!("Skipping test: requires --features \"cli\"");
    }
    
    #[tokio::test]
    async fn test_cli_help() {
        println!("Skipping test: requires --features \"cli\"");
    }
    
    #[tokio::test]
    async fn test_generate_help() {
        println!("Skipping test: requires --features \"cli\"");
    }
    
    #[tokio::test]
    async fn test_validate_help() {
        println!("Skipping test: requires --features \"cli\"");
    }
    
    #[tokio::test]
    async fn test_decrypt_help() {
        println!("Skipping test: requires --features \"cli\"");
    }
    
    #[tokio::test]
    async fn test_cli_version() {
        println!("Skipping test: requires --features \"cli\"");
    }
}
