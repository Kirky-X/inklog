use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::Aes256Gcm;
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

const MAGIC_HEADER: &[u8] = b"ENCLOG1\0";

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum EncryptionVersion {
    V1WithAlgo,
    V1Legacy,
    Unknown,
}

#[allow(dead_code)]
pub fn detect_version(header: &[u8]) -> EncryptionVersion {
    if header.len() < 10 {
        return EncryptionVersion::Unknown;
    }

    if &header[..8] == MAGIC_HEADER {
        let version = u16::from_le_bytes([header[8], header[9]]);
        if version == 1 {
            return EncryptionVersion::V1Legacy;
        }
    }
    EncryptionVersion::Unknown
}

#[allow(dead_code)]
pub fn decrypt_file(input_path: &PathBuf, output_path: &PathBuf, key_env: &str) -> Result<()> {
    let mut file = File::open(input_path)
        .with_context(|| format!("Failed to open input file: {}", input_path.display()))?;

    let mut header = [0u8; 24];
    file.read_exact(&mut header)
        .with_context(|| "Failed to read file header")?;

    if &header[..8] != MAGIC_HEADER {
        return Err(anyhow!("Invalid file header: not an encrypted inklog file"));
    }

    let version = u16::from_le_bytes([header[8], header[9]]);
    if version != 1 {
        return Err(anyhow!("Unsupported file version: {}", version));
    }

    let algo = u16::from_le_bytes([header[10], header[11]]);
    if algo != 1 {
        return Err(anyhow!("Unsupported encryption algorithm: {}", algo));
    }

    let key = get_encryption_key(key_env)
        .with_context(|| format!("Failed to get encryption key from env var: {}", key_env))?;

    let nonce = aes_gcm::Nonce::from_slice(&header[12..24]);

    let mut ciphertext = Vec::new();
    file.read_to_end(&mut ciphertext)
        .with_context(|| "Failed to read ciphertext")?;

    let cipher = Aes256Gcm::new((&key).into());

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;

    let mut output_file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;

    output_file
        .write_all(&plaintext)
        .with_context(|| "Failed to write decrypted data")?;

    Ok(())
}

#[allow(dead_code)]
pub fn decrypt_file_legacy(
    input_path: &PathBuf,
    output_path: &PathBuf,
    key_env: &str,
) -> Result<()> {
    let key = get_encryption_key(key_env)
        .with_context(|| format!("Failed to get encryption key from env var: {}", key_env))?;

    let mut file = File::open(input_path)
        .with_context(|| format!("Failed to open input file: {}", input_path.display()))?;

    let mut header = [0u8; 10];
    file.read_exact(&mut header)
        .with_context(|| "Failed to read file header")?;

    if &header[..8] != MAGIC_HEADER {
        return Err(anyhow!("Invalid file header: not an encrypted inklog file"));
    }

    let version = u16::from_le_bytes([header[8], header[9]]);
    if version != 1 {
        return Err(anyhow!("Unsupported file version: {}", version));
    }

    let mut nonce_bytes = [0u8; 12];
    file.read_exact(&mut nonce_bytes)
        .with_context(|| "Failed to read nonce")?;

    let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);

    let mut ciphertext = Vec::new();
    file.read_to_end(&mut ciphertext)
        .with_context(|| "Failed to read ciphertext")?;

    let cipher = Aes256Gcm::new((&key).into());

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;

    let mut output_file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;

    output_file
        .write_all(&plaintext)
        .with_context(|| "Failed to write decrypted data")?;

    Ok(())
}

pub fn decrypt_file_compatible(
    input_path: &PathBuf,
    output_path: &PathBuf,
    key_env: &str,
) -> Result<()> {
    let mut file = File::open(input_path)
        .with_context(|| format!("Failed to open input file: {}", input_path.display()))?;

    let mut header = [0u8; 24];
    let read_count = file
        .read(&mut header)
        .with_context(|| "Failed to read file header")?;

    if read_count < 10 {
        return Err(anyhow!("File too small to be a valid encrypted file"));
    }

    if &header[..8] != MAGIC_HEADER {
        return Err(anyhow!("Invalid file header: not an encrypted inklog file"));
    }

    let version = u16::from_le_bytes([header[8], header[9]]);
    if version != 1 {
        return Err(anyhow!("Unsupported file version: {}", version));
    }

    let key = get_encryption_key(key_env)
        .with_context(|| format!("Failed to get encryption key from env var: {}", key_env))?;

    let algo = u16::from_le_bytes([header[10], header[11]]);
    let plaintext: Vec<u8>;

    if algo == 1 {
        let nonce_slice: [u8; 12] = header[12..24].try_into().unwrap();
        let nonce = aes_gcm::Nonce::from_slice(&nonce_slice);

        let mut ciphertext = Vec::new();
        file.read_to_end(&mut ciphertext)
            .with_context(|| "Failed to read ciphertext")?;

        let cipher = Aes256Gcm::new((&key).into());
        plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;
    } else if header[10] == 0 && header[11] == 0 && read_count >= 22 {
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes.copy_from_slice(&header[10..22]);
        let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);

        let mut ciphertext = Vec::new();
        file.read_to_end(&mut ciphertext)
            .with_context(|| "Failed to read ciphertext")?;

        let cipher = Aes256Gcm::new((&key).into());
        plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;
    } else {
        return Err(anyhow!("Unsupported encryption format"));
    }

    let mut output_file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;

    output_file
        .write_all(&plaintext)
        .with_context(|| "Failed to write decrypted data")?;

    Ok(())
}

#[allow(dead_code)]
pub fn decrypt_file_to_string(input_path: &PathBuf, key_env: &str) -> Result<String> {
    let temp_output = input_path.with_extension("decrypted");
    decrypt_file(input_path, &temp_output, key_env)?;

    let mut content = String::new();
    File::open(&temp_output)?.read_to_string(&mut content)?;

    std::fs::remove_file(&temp_output)?;

    Ok(content)
}

fn get_encryption_key(env_var: &str) -> Result<[u8; 32]> {
    let key_str = std::env::var(env_var)
        .map_err(|e| anyhow!("Failed to read encryption key from {}: {}", env_var, e))?;

    if let Ok(decoded) = general_purpose::STANDARD.decode(key_str.trim()) {
        if decoded.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&decoded);
            return Ok(key);
        }
    }

    let mut key = [0u8; 32];
    let bytes = key_str.as_bytes();
    if bytes.len() < 32 {
        key[..bytes.len()].copy_from_slice(bytes);
        for (i, key_byte) in key.iter_mut().enumerate().skip(bytes.len()) {
            *key_byte = (i % 256) as u8;
        }
    } else {
        key.copy_from_slice(&bytes[..32]);
    }

    Ok(key)
}

#[allow(dead_code)]
pub fn decrypt_directory(
    input_dir: &PathBuf,
    output_dir: &PathBuf,
    key_env: &str,
    recursive: bool,
) -> Result<()> {
    if !input_dir.exists() {
        return Err(anyhow!(
            "Input directory does not exist: {}",
            input_dir.display()
        ));
    }

    std::fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            output_dir.display()
        )
    })?;

    let entries = std::fs::read_dir(input_dir)
        .with_context(|| format!("Failed to read input directory: {}", input_dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "enc" {
                    let file_name = path.file_name().unwrap();
                    let output_path = output_dir.join(file_name).with_extension("log");

                    println!(
                        "Decrypting: {} -> {}",
                        path.display(),
                        output_path.display()
                    );

                    if let Err(e) = decrypt_file(&path, &output_path, key_env) {
                        eprintln!("Failed to decrypt {}: {}", path.display(), e);
                    }
                }
            }
        } else if recursive && path.is_dir() {
            let sub_output_dir = output_dir.join(path.file_name().unwrap());
            decrypt_directory(&path, &sub_output_dir, key_env, recursive)?;
        }
    }

    Ok(())
}

pub fn decrypt_directory_compatible(
    input_dir: &PathBuf,
    output_dir: &PathBuf,
    key_env: &str,
    recursive: bool,
) -> Result<()> {
    if !input_dir.exists() {
        return Err(anyhow!(
            "Input directory does not exist: {}",
            input_dir.display()
        ));
    }

    std::fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            output_dir.display()
        )
    })?;

    let entries = std::fs::read_dir(input_dir)
        .with_context(|| format!("Failed to read input directory: {}", input_dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "enc" {
                    let file_name = path.file_name().unwrap();
                    let output_path = output_dir.join(file_name).with_extension("log");

                    println!(
                        "Decrypting: {} -> {}",
                        path.display(),
                        output_path.display()
                    );

                    if let Err(e) = decrypt_file_compatible(&path, &output_path, key_env) {
                        eprintln!("Failed to decrypt {}: {}", path.display(), e);
                    }
                }
            }
        } else if recursive && path.is_dir() {
            let sub_output_dir = output_dir.join(path.file_name().unwrap());
            decrypt_directory_compatible(&path, &sub_output_dir, key_env, recursive)?;
        }
    }

    Ok(())
}

pub fn batch_decrypt(input_pattern: &str, output_dir: &PathBuf, key_env: &str) -> Result<()> {
    let paths = glob::glob(input_pattern)
        .map_err(|e| anyhow!("Invalid glob pattern: {}", e))?
        .filter_map(|p| p.ok())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "enc"));

    std::fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            output_dir.display()
        )
    })?;

    for path in paths {
        let file_name = path.file_name().unwrap();
        let output_path = output_dir.join(file_name).with_extension("log");

        println!(
            "Decrypting: {} -> {}",
            path.display(),
            output_path.display()
        );

        if let Err(e) = decrypt_file_compatible(&path, &output_path, key_env) {
            eprintln!("Failed to decrypt {}: {}", path.display(), e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::Aes256Gcm;
    use rand::Rng;
    use std::io::Write;

    const TEST_KEY: [u8; 32] = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
        0x1f, 0x20,
    ];

    fn create_encrypted_file_v1(path: &PathBuf, plaintext: &[u8], key: &[u8; 32]) -> Result<()> {
        let mut file = File::create(path)?;

        file.write_all(MAGIC_HEADER)?;
        file.write_all(&1u16.to_le_bytes())?;
        file.write_all(&1u16.to_le_bytes())?;

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill(&mut nonce_bytes);
        file.write_all(&nonce_bytes)?;

        let cipher = Aes256Gcm::new(key.into());
        let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| anyhow!("Encryption error: {}", e))?;

        file.write_all(&ciphertext)?;

        Ok(())
    }

    fn create_encrypted_file_legacy(
        path: &PathBuf,
        plaintext: &[u8],
        key: &[u8; 32],
    ) -> Result<()> {
        let mut file = File::create(path)?;

        file.write_all(MAGIC_HEADER)?;
        file.write_all(&1u16.to_le_bytes())?;

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill(&mut nonce_bytes);
        file.write_all(&nonce_bytes)?;

        let cipher = Aes256Gcm::new(key.into());
        let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| anyhow!("Encryption error: {}", e))?;

        file.write_all(&ciphertext)?;

        Ok(())
    }

    #[test]
    fn test_magic_header_validation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let invalid_file = temp_dir.path().join("invalid.enc");
        let mut file = File::create(&invalid_file).unwrap();
        let mut invalid_header = [0u8; 24];
        invalid_header[..14].copy_from_slice(b"INVALID_HEADER");
        file.write_all(&invalid_header).unwrap();

        let result = decrypt_file(&invalid_file, &PathBuf::from("output.log"), "TEST_KEY");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Invalid file header"),
            "Expected error about invalid header, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_version_validation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let invalid_file = temp_dir.path().join("invalid_version.enc");
        let mut file = File::create(&invalid_file).unwrap();
        let mut header = [0u8; 24];
        header[..8].copy_from_slice(MAGIC_HEADER);
        header[8..10].copy_from_slice(&999u16.to_le_bytes());
        header[10..12].copy_from_slice(&1u16.to_le_bytes());
        file.write_all(&header).unwrap();

        let result = decrypt_file(&invalid_file, &PathBuf::from("output.log"), "TEST_KEY");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Unsupported file version"),
            "Expected error about unsupported version, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_get_encryption_key_base64() {
        let key_base64 = general_purpose::STANDARD.encode(TEST_KEY);
        std::env::set_var("TEST_ENCRYPTION_KEY", &key_base64);

        let key = get_encryption_key("TEST_ENCRYPTION_KEY").unwrap();
        assert_eq!(key, TEST_KEY);

        std::env::remove_var("TEST_ENCRYPTION_KEY");
    }

    #[test]
    fn test_decrypt_file_v1_format() {
        let temp_dir = tempfile::tempdir().unwrap();
        let input_file = temp_dir.path().join("test_v1.enc");
        let output_file = temp_dir.path().join("test_v1.log");
        let plaintext = b"Hello, World! V1 format test.";

        create_encrypted_file_v1(&input_file, plaintext, &TEST_KEY).unwrap();

        std::env::set_var("TEST_V1_KEY", general_purpose::STANDARD.encode(TEST_KEY));
        let result = decrypt_file(&input_file, &output_file, "TEST_V1_KEY");

        assert!(result.is_ok());

        let mut decrypted_content = String::new();
        File::open(&output_file)
            .unwrap()
            .read_to_string(&mut decrypted_content)
            .unwrap();
        assert_eq!(decrypted_content.as_bytes(), plaintext);

        std::env::remove_var("TEST_V1_KEY");
    }

    #[test]
    fn test_decrypt_file_legacy_format() {
        let temp_dir = tempfile::tempdir().unwrap();
        let input_file = temp_dir.path().join("test_legacy.enc");
        let output_file = temp_dir.path().join("test_legacy.log");
        let plaintext = b"Hello, World! Legacy format test.";

        create_encrypted_file_legacy(&input_file, plaintext, &TEST_KEY).unwrap();

        std::env::set_var(
            "TEST_LEGACY_KEY",
            general_purpose::STANDARD.encode(TEST_KEY),
        );
        let result = decrypt_file_legacy(&input_file, &output_file, "TEST_LEGACY_KEY");

        assert!(result.is_ok());

        let mut decrypted_content = String::new();
        File::open(&output_file)
            .unwrap()
            .read_to_string(&mut decrypted_content)
            .unwrap();
        assert_eq!(decrypted_content.as_bytes(), plaintext);

        std::env::remove_var("TEST_LEGACY_KEY");
    }

    #[test]
    fn test_decrypt_compatible_auto_detect() {
        let temp_dir = tempfile::tempdir().unwrap();
        let input_file = temp_dir.path().join("test_compat.enc");
        let output_file = temp_dir.path().join("test_compat.log");
        let plaintext = b"Auto-detect format test.";

        create_encrypted_file_v1(&input_file, plaintext, &TEST_KEY).unwrap();

        std::env::set_var(
            "TEST_COMPAT_KEY",
            general_purpose::STANDARD.encode(TEST_KEY),
        );
        let result = decrypt_file_compatible(&input_file, &output_file, "TEST_COMPAT_KEY");

        assert!(result.is_ok());

        let mut decrypted_content = String::new();
        File::open(&output_file)
            .unwrap()
            .read_to_string(&mut decrypted_content)
            .unwrap();
        assert_eq!(decrypted_content.as_bytes(), plaintext);

        std::env::remove_var("TEST_COMPAT_KEY");
    }

    #[test]
    fn test_version_detection() {
        let header_v1 = *b"ENCLOG1\0\x01\x00\x01\x00";
        let header_legacy = *b"ENCLOG1\0\x01\x00\x00\x00";
        let header_invalid = *b"INVALID\x00\x01\x00\x00";

        assert_eq!(
            detect_version(&header_v1[..10]),
            EncryptionVersion::V1Legacy
        );
        assert_eq!(
            detect_version(&header_legacy[..10]),
            EncryptionVersion::V1Legacy
        );
        assert_eq!(
            detect_version(&header_invalid[..10]),
            EncryptionVersion::Unknown
        );
    }
}
