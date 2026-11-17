# 🔒 inklog Security Documentation

This document describes inklog's security design (encryption, data masking, memory safety, access control, network security, and compliance), the vulnerability reporting process, and security best practices when using inklog to handle sensitive logs.

> Related documents: [📖 User Guide](USER_GUIDE.md) · [🏗️ Architecture](ARCHITECTURE.md) · [🤝 Contributing Guide](CONTRIBUTING.md)

<details open>
<summary>📑 Contents</summary>

- [📌 Supported Versions and Maintenance Policy](#-supported-versions-and-maintenance-policy)
- [🚨 Vulnerability Reporting Process](#-vulnerability-reporting-process)
- [🛡️ Security Design Overview](#️-security-design-overview)
- [🔐 Encryption and Key Management](#-encryption-and-key-management)
- [🎭 Data Masking](#-data-masking)
- [🧠 Memory Safety](#-memory-safety)
- [🚪 Access Control](#-access-control)
- [🌐 Network Security](#-network-security)
- [📜 Compliance](#-compliance)
- [✅ Security Best Practices](#-security-best-practices)
- [📎 Appendix](#-appendix)

</details>

---

## 📌 Supported Versions and Maintenance Policy

This project is in the 0.x pre-release stage and **maintains only the current minor version line** (`0.3.x`, including rc pre-releases within that line).

- **Current line**: receives defect fixes and security fixes, delivered with the latest release of that line.
- **Older minor lines**: reach EOL as soon as the first version of the next minor line is released; no further fixes of any kind are provided, and security patches are not backported.
- **No LTS**: before 1.0, no cross-minor API compatibility is promised; minor-version upgrades may contain breaking changes — read the [CHANGELOG](../CHANGELOG.md) before upgrading.
- **crates.io visibility ≠ supported**: old versions remain downloadable (unless yanked for severe defects) but are out of the supported scope.

| Version line | Status | Notes |
|--------|------|------|
| `0.3.x` (current) | ✅ supported | the only line receiving fixes |
| `≤ 0.2.x` | ❌ EOL | superseded by the current line; unmaintained — please upgrade |

> Historical security-fix example: 0.1.11 fixed three security issues — `table_name` SQL injection protection, `FileSink` path traversal protection, and HTTP authentication fail-closed (see the [changelog](../CHANGELOG.md)); that line is EOL and will receive no further fixes.

## 🚨 Vulnerability Reporting Process

If you discover a security vulnerability in inklog, please report it to us responsibly.

### Reporting Principles

**❌ Avoid**:

- disclosing security vulnerabilities publicly in GitHub Issues;
- publishing vulnerability details on social media;
- exploiting vulnerabilities without authorization.

**✅ Recommended**:

- report vulnerabilities through private channels;
- provide reproduction steps and an impact assessment;
- allow sufficient time for a fix.

### How to Report

**Preferred**:

- [GitHub Security Advisories](https://github.com/Kirky-X/inklog/security/policy) (repo page Security → Report a vulnerability)

**Alternative**:

- Email: [Kirky-X@outlook.com](mailto:Kirky-X@outlook.com) (handled by Kirky.X; no PGP public key published — do not expect encrypted email)

### What to Include

| Section | Information to include |
|------|--------------|
| **Vulnerability description** | vulnerability type (injection, authentication bypass, etc.), affected scope (affected versions), severity (CVSS score) |
| **Reproduction steps** | detailed reproduction steps, code samples or configuration, expected vs actual behavior |
| **Impact assessment** | likelihood of data leakage, impact on system availability, business impact scope |
| **Mitigation** | temporary mitigations, suggested fix direction |

### ⏱️ Response Commitments

> This repository's response targets are stricter than the seven-repo baseline (baseline: acknowledgment within 48 hours, initial assessment within 7 days); this table is authoritative.

| Stage | Time | Action |
|------|------|------|
| **Acknowledgment** | within 24 hours | confirm receipt of the security report |
| **Initial assessment** | within 48 hours | assess vulnerability severity |
| **Fix development** | 7-14 days | develop and test the fix |
| **Patch release** | after the fix is done | release the security patch |
| **Public disclosure** | 7-30 days after the fix | publish the security advisory (coordinated disclosure) |

### Coordinated Disclosure Process

```text
[Day 0]  Researcher reports the vulnerability
         ↓
[Day 1]   inklog acknowledges and assesses
         ↓
[Day 7]   Fix development complete
         ↓
[Day 10]  Patch released to private preview
         ↓
[Day 14]  Public release + security advisory
```

**Influencing factors**: vulnerability severity (critical vulnerabilities first), fix complexity, known public exploitation.

### Known Security Issues and Subscriptions

To view currently known security issues:

- **GitHub Security Advisories**: <https://github.com/Kirky-X/inklog/security/policy>
- **Changelog**: [CHANGELOG](../CHANGELOG.md)

Subscribe to security updates: on the GitHub repository click **Watch → Custom** and check **Releases** and **Security alerts**.

### Security Resources

**Learning resources**:

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [CWE Top 25](https://cwe.mitre.org/top25/)
- [RustSec Advisory DB](https://github.com/rustsec/advisory-db)

**Recommended tools**:

- `cargo-audit`: dependency vulnerability scanning
- `cargo-deny`: license and source checks

## 🛡️ Security Design Overview

inklog is security-first Rust logging infrastructure designed for environments with strict data-protection and compliance requirements.

### Design Principles

- **Encryption first**: AES-256-GCM authenticated encryption, with key-management service integration;
- **Zero trust**: all sensitive data enters via environment variables or KMS, never hard-coded;
- **Memory safety**: `zeroize` ensures sensitive data leaves no trace in memory;
- **Defense in depth**: multiple layers of security controls, from encryption and masking to access control;
- **Compliance by design**: supports GDPR, HIPAA, PCI-DSS, and similar requirements.

### Security Capabilities Overview

| Capability | Implementation location | Notes |
|------|----------|------|
| Encryption at rest | `support::io::sink::encryption` | AES-256-GCM, applied only to rotated archives |
| Key derivation | same as above | PBKDF2-HMAC-SHA256, 600k iterations, at most once per rotation |
| Key zeroization in memory | `zeroize` | keys are zeroed automatically when they leave scope |
| KMS key providers | `support::security` (`kms` feature) | `EnvKeyProvider` / `ConfersKeyProvider` / Vault transit MVP |
| Data masking | `support::processing::masking` | 21 built-in regex rules (fancy-regex lookaround for CJK-friendly boundaries) + sensitive field-name detection + a custom registry; all egress points (console/file/database/net/otlp) mask by default (`masking_enabled` gate), recursion depth capped at 16 levels and single inputs at 1 MiB |
| Secret outbound masking gate | `support::processing::secret_scan` (`secret-scan` feature) | value-shape regex registry (`sk-` prefix/PEM private-key blocks/AWS AKIA and nine bare-secret classes in total) + value entropy scanning (Shannon, configurable threshold) + per-pattern counting; `mask_checked` fails closed, rejecting output when the limit is exceeded; `mask` closes over oversized inputs (>1 MiB) with whole-value redaction, likewise fail-closed (what cannot be scanned cannot be proven safe); Chinese sensitive key-name group merged into key-name detection |
| Path safety | `validation::path` (PathValidator) | path traversal protection; writing to user home directories and key files is forbidden |
| Content sanitization | `validation::sanitize` (LogSanitizer) | log injection and control-character protection |
| SQL injection protection | DatabaseSink / `integrations::infra::database` | table-name whitelist validation + parameterized queries |
| HTTP access control | `http` feature | auth token cached at startup, fail-closed on failure; IP allowlist; TLS; **rejected at configuration time** for non-loopback binding without auth/allowlist (the exposure guard covers `InklogConfig::validate` and the library's own build entry points) |
| Error text egress masking | `error::InklogError` / `validation::sanitize` | all egress points — `Display` / `Debug` / `localized_message()` / `safe_message()` — are masked: paths, DSNs, SQL fragments, and credentials never leak with error text; the `Debug` view of `LogRecord` is masked while `Serialize` stays verbatim (pre-persistence masking is the sink's responsibility, gated by `masking_enabled` / `secret_scan_enabled`) |
| Outbound TLS authentication | `support::io::sink::net` / `otlp` (`net-sink` / `otlp` feature) | `danger_accept_invalid_certs` (skip certificate verification) is **compile-time gated**: available only in debug builds (`debug_assertions`) or with the explicit `insecure-tls` feature; unauthorized release builds fail at construction (fail-closed); when allowed, the warning goes to standard error (the global subscriber is not yet installed at sink-construction time, so `tracing::warn!` would be dropped) |
| Tamper-evident archival | `support::audit_chain` (ArchiveChain) | HMAC-SHA256 archive chain (random chain-start salt) against deletion, reordering, and forgery; with `audit_chain_enabled = true`, every rotation records `{path, sha256, timestamp}`, write-through to the `<stem>.chain.jsonl` manifest and the `<stem>.chain.seal.json` seal (an out-of-band anchor: total entry count + tail hash + seal HMAC), verified by `inklog-cli verify-chain --manifest <file> --key-env INKLOG_AUDIT_KEY` (exit code 0 when the chain is intact, 2 when tampered; **an empty manifest and a missing seal both count as failure** — in-chain consistency cannot prove "no entries were deleted", so an empty chain and a missing anchor are treated fail-closed; **truncation to a non-empty prefix is detected by the seal**, which in-chain verification cannot detect) |
| Supply chain security | `deny.toml` / lefthook | `cargo deny check` + `cargo audit` + pre-commit private-key scanning |

> Note: the repository maintains [`deny.toml`](../../deny.toml); the CI security job and lefthook pre-push run `cargo deny check` (advisories/licenses/duplicate crates) and `cargo audit` (RustSec advisories) respectively.

### Threat Model Boundaries of the Archive Audit Chain

- **The seal shares the manifest's directory and writable domain**: the seal path (`<stem>.chain.seal.json`) derives from the manifest path; the writer persists them in two separate writes (not an atomic paired publication). Rolling back both a generation of the manifest and the then-valid seal is invisible to `inklog-cli verify-chain` — **the seal only blocks one-sided truncation, not paired rollback**; covering paired rollback requires placing the seal in an independent writable domain (the CLI currently has no `--seal` parameter).
- **Rotation during verification**: the manifest and the seal are two independent reads; a rotation landing between the reads produces an entry-count mismatch. The CLI re-reads once before judging — if the re-read pair matches, it reports "rotation occurred during verification" and returns the exit code for the re-read result; if the re-read still mismatches, it emits a diagnosis distinct from "tampered" (manifest and seal from different generations) while still exiting 2 (fail-closed — a writer-side inconsistency is not misreported as deliberate tampering).
- **Crash residue**: a writer crash between the manifest and the seal leaves a mismatch until the next successful rotation writes both again — verification is treated fail-closed in the meantime.

## 🔐 Encryption and Key Management

inklog uses **AES-256-GCM** (Galois/Counter Mode) for authenticated encryption, providing confidentiality and integrity together.

### Encryption Flow

1. **Key acquisition**: securely read the 32-byte key from an environment variable (wrapped in `Zeroizing`);
2. **Nonce generation**: a cryptographically secure RNG creates a 12-byte nonce;
3. **Encryption**: AES-256-GCM encrypts the data;
4. **Integrity verification**: GCM mode includes the authentication tag automatically;
5. **File write**: the encrypted data is written in a versioned format.

> Encryption applies only to **rotated archives**: the active log file stays in plaintext; rotation triggers compression first (`.zst`) and then background encryption (`.enc`), so encryption and key derivation never occupy the per-record write hot path.

### Key Acquisition

```rust
// src/support/io/sink/encryption.rs
pub fn get_encryption_key(env_var: &str) -> Result<Zeroizing<[u8; 32]>, InklogError>
```

- The 32-byte key wrapped in `Zeroizing` is zeroed automatically when it leaves scope;
- An environment variable holding Base64-encoded 32 bytes is used directly as the key; an ordinary password (1-127 characters) is derived via PBKDF2-HMAC-SHA256 (600,000 iterations);
- Low-entropy or wrong-length keys are rejected (error at startup).

**Supported key formats**:

| Format | Example | Notes |
|------|------|------|
| **Base64** | `MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI=` | recommended, easy to manage |
| **Raw bytes** | 32 bytes of binary data | needs precise handling |
| **Password string** | 1-127 characters | derived via PBKDF2 (supported by the v2 format) |

**Environment variable configuration**:

```bash
# Generate and set the encryption key (recommended)
export INKLOG_ENCRYPTION_KEY=$(openssl rand -base64 32)
```

**Key file (secondary source)**: when the environment variable is unset, the key can be read from a key file (`encryption_key_file` / the journal's `fallback_journal_key_file`); resolution priority is **env > file**. The key file must have unix permissions `0600` — key files readable by group/others are explicitly rejected at resolution time; when neither source is available, the encryption path **fails explicitly** (error at construction) — there is no silent plaintext fallback.

### Encrypted File Format

#### v2 format (current)

```text
Offset    Size     Description
--------  -------  ------------------
0-7       8 bytes  MAGIC: "ENCLOG1\0"
8-9       2 bytes  Version: 2 (u16 little-endian)
10-11     2 bytes  Algorithm ID: 1 (AES-256-GCM)
12-27     16 bytes PBKDF2 salt
28-39     12 bytes Nonce
40+       N bytes  Ciphertext (including the GCM authentication tag)
```

- Two key sources: when the environment variable is Base64/raw 32 bytes, it is used directly as the key (the salt is stored but unused, for format uniformity); for an ordinary password, the header salt drives deterministic PBKDF2-HMAC-SHA256 derivation (600,000 iterations, OWASP-recommended), and the decrypting side recovers it by reading the salt from the file header;
- 600,000 PBKDF2 iterations is the minimum count accepted by security review; lowering it for performance is forbidden (one derivation costs about 63 ms, see the [⚡ Performance Baseline](PERFORMANCE.md), at most once per rotated file).

#### v1 and Legacy formats (backward compatible)

| Format | Header structure | Notes |
|------|----------|------|
| v1 | `[MAGIC 8][version 2][algo 2][nonce 12][ciphertext]` | salt-less, still decryptable; passwords from the v1 era cannot be recovered because no salt was stored (an explicit error is reported at decryption time) — migrate to v2 |
| Legacy | `[MAGIC 8][version 2][nonce 12][ciphertext]` | early format, auto-detected by the decryption tooling |

### fallback journal encryption (optional)

With `global.fallback_journal_encrypt = true`, the disk-persisted fallback journal is encrypted at rest with AES-256-GCM:

```text
Offset    Size     Description
--------  -------  ------------------
0-7       8 bytes  MAGIC: "INKJRN1\0"
8-9       2 bytes  Format version: 1 (u16 little-endian)
10-11     2 bytes  Algorithm ID: 1 (AES-256-GCM)
12-27     16 bytes PBKDF2 salt
Thereafter  per segment  nonce(12) + segment length(4, u32le) + ciphertext (plaintext is single-line JSON)
```

- **Nonce uniqueness policy**: random 96-bit per segment (CSPRNG); unit tests assert no reuse of the same key across segments/files;
- **Explicit failure**: when encryption is enabled but the key is missing/invalid (env and key file both unavailable, wrong format, insufficient raw-key entropy), construction errors out — silent plaintext persistence is forbidden;
- **Format adaptation**: `replay` recognizes old vs new formats by magic — old plaintext JSONL can be replayed by an encrypted instance; unrecognized/undecryptable content (unsupported version or algorithm, a plaintext instance meeting an encrypted file during configuration rollback, all records failing under a wrong key) leaves the file untouched and reports a per-segment count, never silently wiped or destroyed;
- **File permissions**: encrypted-journal header creation, segment appends, and truncate-rebuild all create with unix `0600` (aligned with FileSink encrypted artifacts and key-file constraints); **plaintext journals get the same permission semantics** — the plaintext path is no longer created 0644 via the default umask; existing files are explicitly tightened after persistence (`mode` takes effect only at creation); when the journal's parent directory is newly created it converges to `0700`;
- **Plaintext-mode threat model**: with `fallback_journal_encrypt = false` (the default), the fallback journal persists as **plaintext JSONL** — it concentrates ERROR/FATAL records, and sensitive context (exception messages, field passthrough) can be denser than the main logs; on shared hosts/backup chains this file is as exposed as the main logs, so sensitive deployments should enable encryption or point `fallback_journal_path` at a restricted directory (file permissions are already unified at 0600 and directories at 0700; directory permissions do not change the file's own exposure);
- **Push port**: `fallback_journal_push_addr` optionally pushes spill records to a remote endpoint (disconnect buffering, failure counting, no backpressure on persistence); `TransportSecurity` reserves a TLS extension point — outbound is plaintext today, so deploy in a trusted network segment or provide transport security at the tunnel layer.

### Key Rotation

Key rotation is a critical practice for encryption security.

**Why rotate keys**:

- **Reduce key-leakage risk**: periodically replacing keys limits the blast radius of a leak;
- **Compliance**: standards such as PCI-DSS and HIPAA require periodic rotation;
- **Algorithm evolution**: supports upgrading to stronger algorithms in the future.

**Rotation steps**:

1. **Generate a new key** and configure it under a new environment variable:

```bash
NEW_KEY=$(openssl rand -base64 32)
export INKLOG_ENCRYPTION_KEY_V2=$NEW_KEY
```

2. **Switch the configuration** to point at the new key environment variable (`encryption_key_env = "INKLOG_ENCRYPTION_KEY_V2"`); newly rotated archives use the new key;
3. **Migrate old archives**: decrypt old files with the CLI (supports v1/v2 and zstd/gzip unpacking); re-persist decrypted output with the new key or import it into the database as needed:

```bash
# Decrypt with the old key (supports --batch for bulk and --recursive for directories)
inklog-cli decrypt \
  --input logs/ \
  --output temp_decrypted/ \
  --batch \
  --recursive \
  --key-env INKLOG_OLD_ENCRYPTION_KEY
```

4. **Update the key management system** (e.g. Kubernetes Secrets) and clean up old key references;
5. **Verify**: sample archives and verify decryption.

**Multi-key decryption**: `inklog-cli query` automatically attempts to decrypt `.enc` files while searching, so old and new keys coexisting enables a smooth migration.

**Rotation best practices**:

| Practice | Notes |
|------|------|
| **Rotation period** | recommended every 90 days; high-security environments can shorten to 30 days |
| **Key storage** | use a KMS or Secrets Manager, never hard-code (the `kms` feature provides the Vault transit MVP) |
| **Backup policy** | back up old keys and encrypted files before rotating |
| **Audit logs** | record every key rotation operation |
| **Test verification** | verify decryption works after rotation |

**Rotation checklist**:

- [ ] Generate a new strong key (32 bytes, high entropy)
- [ ] Back up current encrypted files
- [ ] Configure the new key in the environment variable
- [ ] Verify encryption with the new key
- [ ] Migrate old encrypted files
- [ ] Update the key management system
- [ ] Clean up old key references
- [ ] Record the rotation audit log
- [ ] Verify all logs decrypt correctly

### Decryption (CLI tool)

```bash
# Decrypt a single file
inklog-cli decrypt \
  --input logs/app_20260913_143022.log.zst.enc \
  --output logs/app_decrypted.log \
  --key-env INKLOG_DECRYPT_KEY

# Decrypt a directory in bulk
inklog-cli decrypt \
  --input logs/ \
  --output decrypted/ \
  --batch \
  --recursive \
  --key-env INKLOG_DECRYPT_KEY
```

**Decryption verification**:

- Auto-detects the file format (v2 / v1 / Legacy);
- Validates the MAGIC header (`ENCLOG1\0`);
- Checks the algorithm ID (only AES-256-GCM is supported);
- The GCM authentication tag automatically verifies data integrity.

## 🎭 Data Masking

inklog ships comprehensive **PII (Personally Identifiable Information)** detection and masking to keep sensitive data out of logs.

### Built-in Rules (21, executed in priority order)

Rules fall into three priority groups: high (10-25), medium (30-40), low (50-100).

#### High-Priority Rules (10-25)

| # | Rule name | Priority | Match pattern | Masking example |
|---|---------|--------|---------|----------|
| 1 | International phone number | 10 | E.164 format | `+1-202-555-0123` → `+1-***-***-0123` |
| 2 | Credit card number (Luhn) | 15 | Visa/MC/Amex/Discover + Luhn check | `4111111111111111` → `****-****-****-1111` |
| 3 | IPv4 address | 20 | `xxx.xxx.xxx.xxx` | `192.168.1.100` → `***.***.***.100` |
| 4 | MAC address | 19 | `XX:XX:XX:XX:XX:XX` or `XX-XX-XX-XX-XX-XX` | `00:1A:2B:3C:4D:5E` → `00:**:**:**:**:5E` |
| 5 | IPv6 address | 21 | full 8-group form and `::` compressed form (RFC 5952) | `2001:0db8:0000:0000:0000:0000:0000:0001` → `****:****:****:****:****:****:****:0001`; `2001:db8::1` → `****:****:****:****:****:****:****:1` |

> The IPv6 rule matches both the fully grouped form and `::` compressed spellings (including `::1`, `1::`, bare `::`); on a hit it uniformly emits the 8-group form, keeping the real value for the last group (for compressed spellings without a last group, that group is `****` too) and replacing the rest with `****`. The dotted tail of IPv4-mapped forms (e.g. `::ffff:192.0.2.1`) is masked first by the IPv4 rule, and the `::ffff` prefix segment by this rule.

#### Medium-Priority Rules (30-40)

| # | Rule name | Priority | Match pattern | Masking example |
|---|---------|--------|---------|----------|
| 6 | Passport number | 30 | `[EeGg]\d{8}` (E/G prefix + 8 digits) | `G12345678` → `G******78` |
| 7 | SSN (US Social Security number) | 35 | `XXX-XX-XXXX` | `123-45-6789` → `***-**-6789` |
| 8 | DB connection string | 40 | `protocol://user:pass@host/db` | `postgres://admin:secret@localhost/db` → `postgres://admin:***@localhost/db` (only the password segment is replaced) |

#### Low-Priority Rules (50-100)

| # | Rule name | Priority | Match pattern | Masking example |
|---|---------|--------|---------|----------|
| 9 | GitHub Token | 50 | `ghp_/gho_/ghu_/ghs_/ghr_` prefixes | `ghp_xxxx` → `***REDACTED_GITHUB***` |
| 10 | Slack Token | 51 | `xoxb-/xoxp-/xapp-` prefixes | `xoxb-xxxx` → `***REDACTED_SLACK***` |
| 11 | Stripe Key | 52 | `sk_live_/rk_live_/pk_live_` prefixes | `sk_live_xxxx` → `***REDACTED_STRIPE***` |
| 12 | Google API Key | 53 | `AIza` prefix, 35 characters | `AIzaSy...` → `***REDACTED_GOOGLE***` |
| 13 | Private key PEM | 54 | `-----BEGIN ... PRIVATE KEY-----` | PEM block → `***REDACTED_PRIVATE_KEY***` |
| 14 | Email address | 100 | standard email format | `user@example.com` → `**@**.***` |
| 15 | Phone number (China) | 100 | `1[3-9]\d{9}` | `13812345678` → `***-****-****` |
| 16 | ID card number (China) | 100 | 18-digit standalone string | `110101199001011234` → `******1234` |
| 17 | Bank card number | 100 | 16-19 consecutive digits (4+8..11+4) | `6222021234567890123` → `****-****-****-0123` |
| 18 | API key | 100 | `api[_-]?key=...` format (value ≥ 20 chars) | `api_key=abcdefghijklmnopqrstuvwxyz` → `api_key=***REDACTED***` (key name kept; the value is replaced wholesale with a single marker) |
| 19 | AWS credentials | 100 | `AKIA/ABIA/ACCA/ASIA` prefixes | `AKIAIOSFODNN7EXAMPLE` → `***REDACTED***` |
| 20 | JWT Token | 100 | `eyJ.eyJ.sig` format | JWT → `***REDACTED_JWT***` |
| 21 | Generic secret | 100 | token/secret/key/password etc. (value ≥ 16 chars) | `secret=abcdefghijklmnop` → `secret=***REDACTED***` |

> The credit-card rule validates the checksum with the **Luhn algorithm** and masks only numbers passing the check, avoiding false positives on ordinary digit sequences.

### Field-Level Sensitive Detection

Sensitive field-name detection (15 word-boundary regexes, case-insensitive; with `secret-scan` enabled, 1 Chinese key-name family is added):

```rust
static SENSITIVE_FIELD_PATTERNS: LazyLock<Vec<regex::Regex>> = LazyLock::new(|| {
    vec![
        // Authentication credentials
        regex::Regex::new(r"(?i)\b(password|passwd|pwd)\b").unwrap(),
        // token/bearer/auth: the preceding character must be a separator or line start (avoids false hits like "cakey")
        regex::Regex::new(r"(?i)(?:[^a-zA-Z0-9_])?(token|bearer|auth)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(secret|credential)\b").unwrap(),
        // API keys
        regex::Regex::new(r"(?i)\b(api[_-]?key|apikey|api[_-]?secret)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(access[_-]?key|access[_-]?key[_-]?id)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(secret[_-]?key|private[_-]?key|public[_-]?key)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(encryption[_-]?key|decryption[_-]?key|master[_-]?key)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(session[_-]?key|session[_-]?id|session[_-]?token)\b").unwrap(),
        // OAuth related
        regex::Regex::new(r"(?i)\b(oauth|oauth[_-]?token|oauth[_-]?secret)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(jwt(_[a-zA-Z0-9]+)?|bearer[_-]?token)\b").unwrap(),
        // AWS credentials
        regex::Regex::new(r"(?i)\b(aws[_-]?secret|aws[_-]?key|aws[_-]?token|aws[_-]?credentials)\b").unwrap(),
        // Database connections
        regex::Regex::new(r"(?i)\b(database[_-]?url|db[_-]?password|db[_-]?user|connection[_-]?string)\b").unwrap(),
        // Payment and PII
        regex::Regex::new(r"(?i)\b(credit[_-]?card|card[_-]?number|cvv|ssn|social[_-]?security)\b").unwrap(),
        // Client credentials
        regex::Regex::new(r"(?i)\b(client[_-]?secret|client[_-]?id)\b").unwrap(),
        // Other sensitive data
        regex::Regex::new(r"(?i)\b(refresh[_-]?token|pin|pin[_-]?code|two[_-]?factor|totp|backup[_-]?code|recovery[_-]?code)\b").unwrap(),
    ]
});
```

With the `secret-scan` feature enabled, a 16th is added: `(密钥|令牌|密码|口令|凭证|凭据|私钥)$` (a key name ending with a sensitive word counts as a sensitive field).

Quick detection function:

```rust
impl DataMasker {
    pub fn is_sensitive_field(field_name: &str) -> bool {
        SENSITIVE_FIELD_PATTERNS
            .iter()
            .any(|pattern| pattern.is_match(field_name))
    }
}
```

### Custom Rules

Extend flexibly via `MaskRuleBuilder`, `DataMaskerBuilder`, and `MaskRuleRegistry`:

```rust
use inklog::{DataMasker, MaskRule, MaskRuleBuilder, MaskRuleRegistry};

// Create a custom rule
let custom_rule = MaskRule::builder("employee_id")
    .pattern(r"\bEMP-\d{6}\b")
    .replacement("EMP-***")
    .priority(30)
    .build()
    .expect("Invalid pattern");

// Assemble a masker on top of the builtin rules
let masker = DataMasker::builder()
    .add_rule(custom_rule)
    .disable_builtin("bank_card")  // Disable a specific builtin rule
    .build();

let result = masker.mask("Employee: EMP-123456");
// Result: "Employee: EMP-***"
```

Managing rules with `MaskRuleRegistry`:

```rust
use inklog::{MaskRule, MaskRuleBuilder, MaskRuleRegistry};

// Create a registry containing all builtin rules
let mut registry = MaskRuleRegistry::with_builtins();

// Register a custom rule
let custom = MaskRule::builder("custom_pattern")
    .pattern(r"CUSTOM-\d+")
    .replacement("***CUSTOM***")
    .priority(25)
    .build()
    .unwrap();
registry.register(custom).unwrap();

// Enable/disable specific rules
registry.set_enabled("email", false);

// Get currently active rules (sorted by priority)
let active = registry.active_rules();
```

Loading rules from TOML configuration:

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

### Direct API Usage

```rust
use inklog::masking::DataMasker;

let masker = DataMasker::new();

// Mask a string
let masked = masker.mask("Contact admin@example.com at 13812345678");
// Result: "Contact **@**.*** at ***-****-****"

// Recursively mask a JSON structure
use serde_json::json;

let mut data = json!({
    "email": "admin@company.org",
    "phone": "13912345678",
    "password": "secret123",
    "contacts": ["user1@test.com", "13811112222"]
});

masker.mask_value(&mut data);
// The password field is fully masked via sensitive-key-name detection
```

### Enabling via Configuration

```toml
# inklog_config.toml
[global]
masking_enabled = true  # Enable data masking (default: true)
```

```bash
# Environment variables
export INKLOG_GLOBAL_MASKING_ENABLED=true
```

### Secret Outbound Masking Gate (feature `secret-scan`)

Key-name triggering cannot catch **bare secrets without key-name context** (an `sk-` key in log prose, PEM private-key blocks, AWS AKIA shapes, etc.). The `secret-scan` feature (off by default) adds three layers of capability, attached via `DataMasker` at the masking point before sink writes:

- **Value-shape registry** `SecretPatternRegistry`: nine built-in shape regexes (prefix + length + charset, three factors each); custom shapes can be registered;
- **Value entropy scanning** `EntropyScanner`: Shannon-entropy judgment on base64 core-alphabet tokens (default ≥20 characters; configurable threshold, default 4.5 bits/char; the full hex alphabet caps entropy at 4.0, so hash/UUID families are not false-positived). Separated by domain from `encryption::validate_key_entropy` — that one validates encryption key bytes, this one probes log content;
- **Per-pattern counting + fail-closed** `SecretScanGate`: per-pattern hit counts are queryable (`hit_counts`); `mask_checked` returns `Err(InklogError::SecretScanLimit)` and refuses output when any pattern's hits within a single input exceed the gate limit; `mask` closes over limit-exceeding inputs with whole-value `***REDACTED_OVERSIZED***` (backward compatible).

```rust
use inklog::{DataMasker, EntropyScanner, SecretPatternRegistry, SecretScanGate};

let gate = SecretScanGate::new(SecretPatternRegistry::with_builtins())
    .with_entropy(EntropyScanner::new())
    .with_match_limit(64);
let probe = gate.clone();

// Wire in the sink outbound masking point (console/file/net/ring_buffered/database
// all hook in via with_masker; database supported since 0.3.0)
let masker = DataMasker::builder().with_secret_scan(gate).build();

let out = masker.mask_checked("token=sk-proj-abcdefghijklmnopqrstuvwxyz123456")?;
assert_eq!(probe.hit_count("openai_sk"), Some(1));
```

With this feature enabled, `DataMasker` key-name detection also covers Chinese sensitive key names (keys ending with `密码`/`令牌`/`密钥`/`口令`/`凭证`/`凭据`/`私钥`, such as `数据库密码`, `访问令牌`; suffix anchoring excludes general words like `密码学`).

#### Wiring and Semantic Boundaries (required reading)

- **Declarative enabling (since 0.3.0)**: `console_sink.secret_scan_enabled` and
  `database_sink.secret_scan_enabled` (both default `false`) hook `secret_scan_masker()` (built-in PII rules + the outbound gate) into the built-in sink assembly path —
  no more manual assembly of custom sinks; the builder equivalents are `LoggerBuilder::console_secret_scan(bool)` /
  `LoggerBuilder::database_secret_scan(bool)`. When the `secret-scan` feature is off,
  the switch is ignored with a warning at the validate stage. Note that the console switch composes with
  `pii_masking_enabled` (if PII masking is off, there is no masking at all); the database side previously
  had no `with_masker` injection point and its masker was hard-wired without the gate — both are filled in by this version (default behavior unchanged bit for bit).
- **Sink egress masking is fail-closed**: every in-crate sink egress path (database/console/file/ring_buffered_file/net/otlp) calls `DataMasker::mask()` — hits are replaced as usual; **situations that cannot be proven safe no longer pass the original text through**:
  - Oversized inputs (> 1 MiB) skip scanning and are redacted wholesale as `***REDACTED_OVERSIZED***` (what cannot be scanned cannot be proven safe; passing it through would persist any credentials/PII inside in plaintext);
  - Regex engine errors (e.g. `fancy-regex` backtracking budget exhausted) redact wholesale as `***REDACTED_ENGINE_ERROR***` with a warn for the record (under an engine error, "did this rule hit" is undecidable);
  - Redaction markers no longer exempt the whole message from rules: marker protection moves down to the **value** level (values already equal to `***REDACTED***`/`***MASKED***`/`[REDACTED]` are not masked again), while bare secrets and PII coexisting in the message are masked as usual — a forged marker cannot exempt the whole record.
  `mask_checked()` still offers the stricter fail-closed exit (returns `Err` when hits exceed the gate or the limit; consumers opt in explicitly).
- **Oversized inputs (> 1 MiB) fork between the two exits**: `mask` skips scanning and redacts wholesale (with a warn — 21 linear scans over oversized text are a CPU amplifier); `mask_checked` returns `Err(InklogError::SecretScanOversizedInput)` and refuses output — what cannot be scanned cannot be proven safe, and the fail-closed exit grants no exemption.
- **Redaction markers no longer turn off the detection face**: neither the gate (`SecretScanGate`) nor `DataMasker::mask()` short-circuits an **entire message** on `***REDACTED`-style markers — the marker text itself is not hit by built-in patterns (idempotence holds naturally), bare secrets in the same message are still replaced and counted, and forged markers cannot exempt the whole record. Marker protection applies only at the **value** level (a value that is already a marker is not masked again, preventing REDACTED wrapping REDACTED). Callers relying on the gate to detect content beside markers can use `SecretScanGate::mask`/`mask_checked` or `DataMasker::detect` directly (the detection face never short-circuits).
- **ReDoS responsibility for custom patterns**: the gate runs on every outbound record, and the underlying `fancy-regex` is a backtracking engine; `SecretPatternRegistry::register` performs no complexity validation, so custom patterns must avoid nested unbounded quantifiers and ambiguous-charset backtracking (see the `register` documentation).

## 🧠 Memory Safety

inklog relies on Rust's memory-safety guarantees and the `zeroize` crate to ensure sensitive data leaves no trace in memory.

### Zeroize Integration

Encryption keys are wrapped in `Zeroizing` throughout (see `get_encryption_key`) and zeroed automatically when they leave scope:

```rust
use zeroize::Zeroizing;

// Env values / derived keys are zeroized automatically
let key = Zeroizing::new(std::env::var("INKLOG_ENCRYPTION_KEY")?);
// ... use the key ...
// key memory is zeroized automatically when it goes out of scope
```

**Cleanup guarantees**:

- **Automatic cleanup**: `Zeroizing` zeroes itself at scope end;
- **Manual cleanup**: call `.zeroize()` to clean immediately;
- **Deterministic overwrite**: memory is overwritten rather than merely freed; compiler optimizations cannot skip the cleanup.

### unsafe Usage Notes

- Production code paths barely use `unsafe`: the only FFI is the Windows disk-space query (`GetDiskFreeSpaceExW`, used by FileSink disk-space management);
- A small number of `unsafe` blocks exist in test code, satisfying edition 2024's unsafe marking requirements for `std::env::set_var` / `std::env::remove_var`;
- All other memory safety is guaranteed by the Rust compiler: no C-style pointer manipulation, no manual memory management, no buffer-overflow risk.

## 🚪 Access Control

inklog enforces strict access control at the file, database, and network layers.

### File Access Control

**Path traversal protection** (`validation::path` PathValidator):

- Validates that log paths are not in sensitive locations such as system directories, user home directories, or key files;
- Decodes `%XX` encodings, backslashes, and control characters to prevent encoding-based bypasses;
- On Unix, validated log files are created with `O_NOFOLLOW | O_CLOEXEC` and **0600 permissions**, rejecting symlink swaps at the kernel level (closing the validate-then-use race); non-Unix platforms degrade to plain creation;
- The CLI decryption tool canonicalizes input paths and verifies they sit inside the base directory, preventing `../../../etc/passwd`-style attacks.

**Disk space management** (FileSink):

- Insufficient-free-space determination: the reserve is the larger of 50MB and 10% of free space; when free space is at or below that reserve, the disk is judged insufficient and the sink degrades (the check is throttled with a cache; no throttling while space is insufficient, re-checked immediately after recovery);
- Automatic old-log cleanup: size-based cleanup (deleting the oldest files until below `max_total_size`) and age-based cleanup (deleting files older than `retention_days`) run independently — when `max_total_size` is unparseable, expiry cleanup still applies; both branches always keep the newest `keep_files` rotated files;
- Compliance scenarios: with `encrypt=true` and `retention_days < 180`, a retention-compliance notice is emitted (等级保护 2.0 requires logs to be retained ≥ 6 months).

**File permissions (audit hardening)**:

- FileSink active log files are created with `0600` (previously only CLI decryption output was 0600);
- Rotated encrypted artifacts (`.enc`) and compressed artifacts (`.zst`/`.gz`) are likewise created with `0600`;
- Newly created log directories get `0700` (existing directories are not re-permissioned).

**Crash consistency (optional)**:

- With `FileSinkConfig.fsync = true`, `sync_all` runs after every batch is written to disk (off by default, throughput first);
- Idle-period fallback flushing: after logs stop flowing, batch buffers are persisted by the rotation timer thread instead of lingering in memory.

### Database Access Control

**SQL injection protection**:

- Table-name whitelist validation (`validate_table_name`): only letters, digits, and underscores; must start with a letter or underscore (`^[a-zA-Z_][a-zA-Z0-9_]*$`, no length cap);
- Business data is always written through parameterized queries; string concatenation is forbidden;
- DDL (table/partition creation) is built from validated names;
- **Escaping semantics follow the actual backend**: the four `DbNexusAdapter` constructors (`with_full_config` / `from_pool` / `from_connection_pool`) resolve the driver with the same criterion — dbnexus's `DbConfig::database_type()` (same source as dbnexus's own pool-backend selection; scheme case-insensitive; `MYSQL://` and `mysql://` are synonyms); when the URL scheme is undecidable or the backend is unsupported (Ladybug/Neo4j), construction fails outright rather than guessing a semantics. The table-creation dialect and the write escaping share that driver's source and never fork. MySQL treats backslashes as escapes by default, so doubling only single quotes can still escape a string literal (backslash + single quote → closed literal + standalone statement); escaping with the wrong backend's semantics becomes an injection surface;
- **DSN credentials never leak**: both `Debug` and `Serialize` of `DatabaseSinkConfig` mask the password inside the DSN (`postgres://user:***@host/db`) — printing or echoing the configuration object as a whole (management-API JSON) carries no database credentials. Masking covers userinfo, query/fragment, and keyword/value forms (credential field names password/pwd/pass/token/secret/api_key/apikey/auth/key/sig/signature, case-insensitive); when the form is undecidable, suspected plaintext passwords are never echoed. A DSN whose password slot is the placeholder (`***`) can never authenticate, and configuration validation rejects that form explicitly.

**Connection pool management**:

```rust
let mut opt = ConnectOptions::new(url);
opt.max_connections(pool_size)                 // Max connection limit
   .min_connections(2)                         // Min connections
   .connect_timeout(Duration::from_secs(5))    // Connect timeout
   .idle_timeout(Duration::from_secs(8));      // Idle timeout
```

- **Connection-count limits**: protects against resource-exhaustion attacks;
- **Timeout protection**: avoids long-hanging connections;
- **Connection reuse**: reduces authentication overhead.

**Principle of least privilege** (recommended database users):

```sql
-- PostgreSQL: create a dedicated log user and grant minimal privileges
CREATE USER inklog_writer WITH PASSWORD 'secure_password';
GRANT CONNECT ON DATABASE logs TO inklog_writer;
GRANT USAGE ON SCHEMA public TO inklog_writer;
GRANT SELECT, INSERT ON ALL TABLES IN SCHEMA public TO inklog_writer;
GRANT CREATE ON SCHEMA public TO inklog_writer;  -- Partition creation (if needed)
```

```sql
-- MySQL
CREATE USER 'inklog_writer'@'%' IDENTIFIED BY 'secure_password';
GRANT INSERT, SELECT ON logs.* TO 'inklog_writer'@'%';
GRANT CREATE ON logs.* TO 'inklog_writer'@'%';  -- Partition creation
```

**SQLite**: filesystem permission control (0600), local access only.

### HTTP Endpoint Access Control (`http` feature)

| Mechanism | Notes |
|------|------|
| **Auth token** | injected via environment variable, cached at startup, fail-closed when acquisition fails (no more degrading to unauthenticated) |
| **IP allowlist** | `ip_whitelist` restricts access sources |
| **TLS** | `TlsConfig` (cert_path / key_path, rustls) enables HTTPS |
| **Error mode** | `Strict` fails startup with an error / `Warn` logs a warning and continues |
| **Binding constraint (fail-closed)** | binding a non-loopback address (`0.0.0.0` / `::` / intranet IP) with neither `auth` nor `ip_whitelist` returns `Err` from configuration validation — `/metrics` and `/health` must not be exposed without access control |
| **Host validation (DNS rebinding)** | when the peer address is loopback, the target host must be a loopback host (`localhost` / `127.0.0.1` / `[::1]`), otherwise 403 — blocking a malicious site from rebinding to a local port and cross-origin reading of endpoint responses. The target host comes from the `Host` header, falling back to the URI authority when absent (HTTP/2 only has `:authority`); when both exist they must point at the same host |
| **Response surface** | `/health` only echoes the status category: sink error originals (including table/constraint names and paths) never enter serialized output; they stay in-process and in internal logs |

## 🌐 Network Security

inklog enforces strict network security for database and forwarding-sink communication.

### Secure Database Connections

**PostgreSQL SSL modes**:

```rust
let url = "postgres://user:pass@localhost/logs?sslmode=require".to_string();
// sslmode options:
// - disable: disable SSL (not recommended)
// - allow: try SSL first, no encryption on failure
// - prefer: try SSL first, plaintext on failure
// - require: SSL required (recommended)
// - verify-ca: verify the CA certificate
// - verify-full: verify CA and hostname (most secure)
```

**MySQL SSL**:

```rust
let url = "mysql://user:pass@localhost/logs?ssl_mode=REQUIRED".to_string();
```

**SQLite**: local file access, controlled by file permissions.

**Connection timeouts**: connect timeout 5 s, idle timeout 8 s, maximum connection lifetime 1 hour, connection-acquisition timeout 30 s.

### Network Forwarding Sink Security (`net-sink` feature)

- **TCP**: optional TLS (rustls client), disconnect buffering and half-open probing;
- **UDP**: NDJSON datagrams, suited to low-sensitivity, high-throughput scenarios;
- For sensitive log forwarding, prefer TCP + TLS.

### Firewall Rule Recommendations

```bash
# PostgreSQL pg_hba.conf (allow only the app server)
hostssl logs    inklog_writer    10.0.1.0/24    scram-sha-256
```

## 📜 Compliance

inklog's security design supports multiple compliance requirements, including GDPR, HIPAA, and PCI-DSS.

### GDPR (General Data Protection Regulation)

#### Personal Data Processing

Automatic PII masking covers:

- ✅ Email addresses (email masking)
- ✅ Phone numbers (Chinese mobile + international E.164)
- ✅ Identity numbers (Chinese ID card, US SSN, passport numbers)
- ✅ Network identifiers (IPv4/IPv6 addresses, MAC addresses)
- ✅ Financial credentials (credit card numbers + Luhn validation, bank card numbers)
- ✅ Third-party tokens (GitHub/Slack/Stripe/Google API keys)
- ✅ Private-key PEM certificates
- ✅ Database connection strings

#### Data Subject Rights

| Right | How inklog supports it |
|------|------------------|
| **Right to be forgotten** | file side: `retention_days` / `max_total_size` / `cleanup_interval_minutes` clean up expired logs automatically; database side: delete expired records per the partition strategy or by time filter |
| **Access and portability** | search and export logs via `inklog-cli query` by time/level/keyword; database side queries by time range (structured fields stored in the `fields` JSON) |
| **Data minimization** | global level control (e.g. `warn` level skips DEBUG/TRACE) + automatic PII masking |

#### Data Protection Measures

- ✅ **Encrypted storage** (GDPR Article 32): AES-256-GCM encryption at rest;
- ✅ **Encrypted transport**: HTTPS/TLS (HTTP endpoints, TCP sink, database SSL);
- ✅ **Key management service integration** (`kms` feature).

### HIPAA (Health Insurance Portability and Accountability Act)

#### Protected Health Information (PHI) Protection

PHI fields (patient IDs, diagnoses, medications, etc.) can be covered by custom masking rules:

```rust
use inklog::MaskRule;

// Register custom masking rules for PHI fields
let patient_rule = MaskRule::builder("patient_id")
    .pattern(r"\bPID-\d{8}\b")
    .replacement("PID-***")
    .priority(30)
    .build()
    .expect("Invalid pattern");
```

#### Audit and Technical Safeguards

**Audit events** (via `publish_ops_event` and structured logs):

- Sensitive data access
- Key usage
- Log export operations
- Encryption/decryption operations

**Technical safeguards**:

| Category | Measure |
|------|------|
| **Access control** | least-privilege database users, HTTP authentication + IP allowlist |
| **Transport security** | TLS 1.2+ database connections and the TCP sink |
| **Physical and environmental security** | network isolation (deployment side) |
| **Integrity** | HMAC-SHA256 tamper-evident archive chain (ArchiveChain) |

### PCI-DSS (Payment Card Industry Data Security Standard)

#### Payment Card Data Protection

```rust
// ✅ Automatically mask credit card numbers (only Luhn-valid numbers are masked)
log::info!("Payment processed: card_number=6222021234567890123");
// Output: Payment processed: card_number=****-****-****-0123
```

#### PCI-DSS Requirement Mapping

| Requirement | inklog implementation |
|------|-------------|
| **Do not store full PANs** | credit card numbers masked automatically (only the last 4 digits kept) |
| **Encrypted transmission** | HTTPS/TLS |
| **Encrypted storage** | AES-256-GCM |
| **Access control** | file permissions 0600, least-privilege database users, HTTP authentication |
| **Log monitoring** | structured logs + Prometheus metrics + the tamper-evident archive chain |
| **Regular vulnerability scanning** | `cargo deny check` + `cargo audit` (CI and pre-push) |

#### Payment-Log Best Practices

**Never log**:

- ❌ CVV/CVC codes
- ❌ PINs
- ❌ Full magnetic-stripe data
- ❌ Full card numbers (record only the last 4 digits)

```rust
// ✅ Correct approach
log::info!("Payment authorized: card_last4=0123, amount=100.00");

// ❌ Wrong approach (the automatic masking fallback catches it, but do not rely on that)
log::info!("Payment: card_number=6222021234567890123, cvv=123");
```

### Compliance Checklists

**GDPR**:

- [ ] Enable data masking (`INKLOG_GLOBAL_MASKING_ENABLED=true`)
- [ ] Configure data retention (`retention_days`)
- [ ] Enable log encryption (`encrypt = true`)
- [ ] Clean up expired logs regularly (`cleanup_interval_minutes` / database partition cleanup)
- [ ] Audit data access (enable `tracing` debug level)

**HIPAA**:

- [ ] Restrict database access (dedicated users)
- [ ] Enable audit logging (tracing debug level)
- [ ] Mask PHI fields (custom rules)
- [ ] Configure network isolation

**PCI-DSS**:

- [ ] Enable credit-card masking (built-in rule, on by default)
- [ ] Never log CVV/CVC codes (code review)
- [ ] Use TLS 1.2+ database connections
- [ ] Run security scans regularly (`cargo deny check` / `cargo audit`)
- [ ] Configure log rotation and archiving

## ✅ Security Best Practices

### 1. Key Management

**Recommended**:

```bash
# Use a .env file (do not commit it to git)
echo "INKLOG_ENCRYPTION_KEY=$(openssl rand -base64 32)" >> .env
echo ".env" >> .gitignore
```

- Rotate keys regularly (recommended every 90 days; process under "Key Rotation" above);
- Enable the `kms` feature in high-security environments (Vault transit MVP);
- Never print keys in logs and never use weak keys (low-entropy keys are rejected at startup validation).

### 2. File and Directory Security

```bash
# Sensitive files 0600, directories 0700 (verification log files created by inklog already default to 0600)
chmod 600 /etc/inklog/config.toml
chmod 700 /var/log/inklog/
```

**Secure directory layout**:

```text
/var/log/inklog/
├── app.log                     # Active log
├── app_20260913_143022.log.zst.enc  # Encrypted archive
└── backup/                     # Rotation backup
    └── 2026-09/
```

### 3. Database Security

- Use dedicated least-privilege users (see "Database Access Control");
- Enforce SSL connections (`sslmode=verify-full` / `ssl_mode=REQUIRED`);
- Enable connection timeouts and connection-count limits.

### 4. Log Configuration Security

```toml
[global]
level = "info"           # No DEBUG/TRACE in production
masking_enabled = true   # Enabled by default

[file_sink]
encrypt = true           # Enable rotated-archive encryption
retention_days = 30      # Cap the retention period
```

### 5. Dependency Security

```bash
cargo deny check advisories   # Known vulnerabilities
cargo deny check bans         # Duplicate dependencies
cargo deny check licenses     # Licenses
cargo audit                   # RustSec advisories
```

On the application side, commit `Cargo.lock` to pin exact versions.

### 6. Monitoring and Auditing

```rust
use inklog::LoggerManager;

let logger = LoggerManager::with_config(config).await?;

// Check log system health periodically
let health = logger.get_health_status();
if let Some(file_health) = health.sinks.get("file") {
    if !file_health.status.is_operational() {
        eprintln!("WARNING: File sink is unhealthy!");
    }
}
```

In production, enable the `http` feature and scrape metrics such as `inklog_sink_healthy` via Prometheus for continuous monitoring.

### 7. Failure Handling

- Database failures degrade automatically through the DB → File → Console three-level fallback; recovery is handled automatically by the health-check thread (see the [🏗️ Architecture](ARCHITECTURE.md) "Failure Fallback and Self-Healing" section);
- Error messages never contain keys or sensitive data (fixed i18n message templates + the masking pipeline as backstop).

## 📎 Appendix

### A. Complete Security Configuration Example (inklog_config.toml)

```toml
[global]
level = "info"
format = "{timestamp} [{level}] {target} - {message}"
masking_enabled = true  # Enable data masking

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
encrypt = true                            # Enable encryption
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

**Environment variable example (.env, never committed)**:

```bash
# Global configuration
INKLOG_GLOBAL_LEVEL=info
INKLOG_GLOBAL_MASKING_ENABLED=true

# File encryption
INKLOG_ENCRYPTION_KEY=$(openssl rand -base64 32)

# Database connection
INKLOG_DATABASE_SINK_URL=postgres://inklog_writer:${DB_PASSWORD}@localhost/logs?sslmode=require
INKLOG_DATABASE_SINK_POOL_SIZE=10

# Decryption key
INKLOG_DECRYPT_KEY=$INKLOG_ENCRYPTION_KEY
```

### B. CLI Security Command Cheat Sheet

```bash
# Generate a config template
inklog-cli generate --config-type full --output ./config/

# Validate the config
inklog-cli validate --config ./config/inklog_config.toml

# Decrypt log files
inklog-cli decrypt \
  --input logs/encrypted.log.zst.enc \
  --output logs/decrypted.log \
  --key-env INKLOG_DECRYPT_KEY

# Decrypt a directory in bulk (recursive)
inklog-cli decrypt \
  --input logs/ \
  --output decrypted/ \
  --batch \
  --recursive \
  --key-env INKLOG_DECRYPT_KEY

# Search local logs (auto decrypt/unpack)
inklog-cli query --path logs/ --level error --since 2026-09-13T00:00:00Z
```

### C. Security Check Commands

```bash
# 1. Run tests (CI scope; mutually exclusive DB backends mean --all-features does not apply)
cargo test --workspace --features "sqlite http cli kit compression gzip parquet fast-masking test-utils"

# 2. Clippy and format checks
cargo clippy --all-targets --features "sqlite http cli kit compression gzip parquet fast-masking" -- -D warnings
cargo fmt --all -- --check

# 3. Security audits
cargo deny check advisories
cargo deny check bans
cargo deny check licenses
cargo audit

# 4. Code coverage (gate ≥ 95%)
cargo llvm-cov --features "sqlite http cli kit compression gzip parquet fast-masking" --lib --fail-under-lines 95

# 5. Check log file permissions
find logs/ -type f -exec chmod 600 {} \;
find logs/ -type d -exec chmod 700 {} \;
```

### D. Glossary

| Term | Definition |
|------|------|
| **AES-256-GCM** | Advanced Encryption Standard, 256-bit, in Galois/Counter Mode |
| **Nonce** | a cryptographic random number guaranteeing uniqueness during encryption |
| **PBKDF2** | password-based key derivation function (inklog uses HMAC-SHA256, 600,000 iterations) |
| **Zeroize** | the technique of securely zeroing memory |
| **PII** | Personally Identifiable Information |
| **PHI** | Protected Health Information |
| **KMS** | Key Management Service |
| **TLS** | Transport Layer Security |
| **GDPR** | General Data Protection Regulation |
| **HIPAA** | Health Insurance Portability and Accountability Act |
| **PCI-DSS** | Payment Card Industry Data Security Standard |

### E. Common Security Issues

**Issue 1**: decryption fails with "Authentication failed"

**Cause**: key mismatch or file corruption.

**Solution**:

```bash
# Verify the key env var contents (should be a 44-char Base64 string)
echo -n "$INKLOG_DECRYPT_KEY" | wc -c

# Confirm the same key as at encryption time is used
export INKLOG_DECRYPT_KEY=$INKLOG_ENCRYPTION_KEY
```

**Issue 2**: file permission error "Permission denied"

**Solution**:

```bash
# Fix file permissions
chmod 600 logs/*.enc
chown $(whoami):$(whoami) logs/*.enc
```

**Issue 3**: startup rejects the key

**Cause**: the key is not 32 bytes (after Base64 decoding) or its entropy is too low.

**Solution**: regenerate the key with `openssl rand -base64 32`.

---

**Document version**: 2.1
**Maintainer**: Kirky.X
**Contact us**: [Kirky-X@outlook.com](mailto:Kirky-X@outlook.com)

*This document is licensed under CC BY-SA 4.0*
