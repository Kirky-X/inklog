// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! T014 端到端：轮转 + 独立密钥锚 + failover 标记的完整 verify-chain 路径。
//!
//! 覆盖 change audit-anchor-failover-integrity 的核心安全不变量：
//! 持链密钥者重写尾部条目并重签 seal、但沿用旧锚 → verify-chain 定位到
//! 首个失配区间，json anchors_valid=="0/N"，退出码非 0。

use assert_cmd::Command;
use inklog::support::audit_chain::{
    ArchiveChain, HmacAnchorSigner, anchor_path_for_manifest, create_anchor_record,
    seal_path_for_manifest,
};
use std::fs;
use tempfile::TempDir;

const CHAIN_KEY: &[u8] = b"t014-chain-key";
const ANCHOR_KEY: &[u8] = b"t014-anchor-key";

/// 构造 manifest + seal + 锚文件（与 file.rs 写入侧同构）。
fn seed_chain(dir: &TempDir, stem: &str) -> (std::path::PathBuf, Vec<String>) {
    let mut chain = ArchiveChain::new(CHAIN_KEY);
    let events = [
        r#"{"path":"r1.log","sha256":"aa","timestamp":"t"}"#,
        r#"{"op":"failover_begin","sink":"r1.log","ts":1}"#,
        r#"{"path":"r2.log","sha256":"bb","timestamp":"t"}"#,
        r#"{"op":"failover_end","sink":"r2.log","ts":2}"#,
    ];
    for event in events {
        chain.append(event);
    }
    let manifest = dir.path().join(format!("{stem}.chain.jsonl"));
    let body = chain
        .entries()
        .iter()
        .map(|e| serde_json::to_string(e).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&manifest, body).unwrap();
    fs::write(
        seal_path_for_manifest(&manifest),
        serde_json::to_string(&chain.seal().unwrap()).unwrap(),
    )
    .unwrap();

    let signer = HmacAnchorSigner::new(ANCHOR_KEY.to_vec());
    let records: Vec<String> = chain
        .entries()
        .iter()
        .enumerate()
        .map(|(i, e)| {
            serde_json::to_string(
                &create_anchor_record(&signer, i as u64, &e.hmac, i as u64 + 1).unwrap(),
            )
            .unwrap()
        })
        .collect();
    fs::write(anchor_path_for_manifest(&manifest), records.join("\n")).unwrap();
    (manifest, records)
}

#[test]
fn verify_chain_with_anchors_and_failover_markers() {
    let dir = TempDir::new().unwrap();
    let (manifest, _) = seed_chain(&dir, "e2e");

    let mut cmd = Command::cargo_bin("inklog-cli").unwrap();
    let assert = cmd
        .env("T014_CHAIN_KEY", "t014-chain-key")
        .env("T014_ANCHOR_KEY", "t014-anchor-key")
        .arg("verify-chain")
        .arg("--manifest")
        .arg(&manifest)
        .arg("--key-env")
        .arg("T014_CHAIN_KEY")
        .arg("--anchor-key-env")
        .arg("T014_ANCHOR_KEY")
        .arg("--json")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json_line = stdout
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with('{'))
        .expect("verify-chain --json must emit a JSON line");
    let json: serde_json::Value = serde_json::from_str(json_line).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["anchor_status"], "ok");
    assert_eq!(json["anchors_valid"], "4/4");
    assert_eq!(json["failover_markers"], 2);
}

#[test]
fn verify_chain_detects_tail_rewrite_reusing_old_anchors() {
    let dir = TempDir::new().unwrap();
    let (manifest, _) = seed_chain(&dir, "tampered");

    // 攻击者持链密钥：重写尾部事件并重签 seal，沿用旧锚。
    let key: Vec<u8> = CHAIN_KEY.to_vec();
    let mut rebuilt = ArchiveChain::new(&key);
    let events = [
        r#"{"path":"r1.log","sha256":"aa","timestamp":"t"}"#,
        r#"{"op":"failover_begin","sink":"r1.log","ts":1}"#,
        r#"{"path":"r2.log","sha256":"bb","timestamp":"t"}"#,
        r#"{"op":"failover_end","sink":"evil.log","ts":2}"#,
    ];
    for event in events {
        rebuilt.append(event);
    }
    fs::write(
        &manifest,
        rebuilt
            .entries()
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();
    fs::write(
        seal_path_for_manifest(&manifest),
        serde_json::to_string(&rebuilt.seal().unwrap()).unwrap(),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("inklog-cli").unwrap();
    let assert = cmd
        .env("T014_CHAIN_KEY", "t014-chain-key")
        .env("T014_ANCHOR_KEY", "t014-anchor-key")
        .arg("verify-chain")
        .arg("--manifest")
        .arg(&manifest)
        .arg("--key-env")
        .arg("T014_CHAIN_KEY")
        .arg("--anchor-key-env")
        .arg("T014_ANCHOR_KEY")
        .arg("--json")
        .assert()
        .failure();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json_line = stdout
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with('{'))
        .expect("verify-chain --json must emit a JSON line");
    let json: serde_json::Value = serde_json::from_str(json_line).unwrap();
    assert_eq!(json["anchor_status"], "fail");
    assert_eq!(json["anchors_valid"], "0/4");
}
