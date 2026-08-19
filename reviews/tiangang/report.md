# Security audit report

Target: `/home/kirky/projects/base/inklog`  
Languages scanned: rust  
Generated: 2026-08-19T17:15:53.114407+00:00

## Summary

| Severity | Count |
|---|---|
| high | 92 |
| medium | 1 |
| **Total** | **93** |

## Tools attempted but failed

These tools were invoked but exited non-zero — their coverage is missing from the findings below. Treat the report as partial; investigate the failures before relying on a clean result.

- **cargo-audit** (exit 1): ety"],"keywords":["concurrency"],"cvss":null,"informational":"unsound","references":[],"source":null,"url":"https://github.com/smol-rs/event-listener/pull/163","withdrawn":null,"license":"CC0-1.0","expect-deleted":false},"affected":null,"versions":{"patched":[">=5.4.2"],"unaffected":["<5.1.0"]}}]}}


## Findings

### High

- **[gitleaks:generic-api-key]** `.secrets.baseline:130` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.808695)
- **[gitleaks:generic-api-key]** `.secrets.baseline:137` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.7561984)
- **[gitleaks:generic-api-key]** `.secrets.baseline:140` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.808695)
- **[gitleaks:generic-api-key]** `.secrets.baseline:144` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.808695)
- **[gitleaks:generic-api-key]** `.secrets.baseline:147` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.7561984)
- **[gitleaks:generic-api-key]** `.secrets.baseline:151` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.7561984)
- **[gitleaks:generic-api-key]** `.secrets.baseline:154` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.808695)
- **[gitleaks:generic-api-key]** `.secrets.baseline:160` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.5848296)
- **[gitleaks:generic-api-key]** `.secrets.baseline:161` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.7561984)
- **[gitleaks:generic-api-key]** `.secrets.baseline:169` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.8495817)
- **[gitleaks:generic-api-key]** `.secrets.baseline:170` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.5848296)
- **[gitleaks:generic-api-key]** `.secrets.baseline:176` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.722574)
- **[gitleaks:generic-api-key]** `.secrets.baseline:179` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.8495817)
- **[gitleaks:generic-api-key]** `.secrets.baseline:183` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.7134607)
- **[gitleaks:generic-api-key]** `.secrets.baseline:186` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.722574)
- **[gitleaks:generic-api-key]** `.secrets.baseline:190` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.5848296)
- **[gitleaks:generic-api-key]** `.secrets.baseline:193` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.7134607)
- **[gitleaks:generic-api-key]** `.secrets.baseline:197` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.6775672)
- **[gitleaks:generic-api-key]** `.secrets.baseline:200` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.5848296)
- **[gitleaks:generic-api-key]** `.secrets.baseline:204` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.853056)
- **[gitleaks:generic-api-key]** `.secrets.baseline:207` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.6775672)
- **[gitleaks:generic-api-key]** `.secrets.baseline:211` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.7561984)
- **[gitleaks:generic-api-key]** `.secrets.baseline:214` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.853056)
- **[gitleaks:generic-api-key]** `.secrets.baseline:220` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.5848296)
- **[gitleaks:generic-api-key]** `.secrets.baseline:221` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.7561984)
- **[gitleaks:generic-api-key]** `.secrets.baseline:227` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.853056)
- **[gitleaks:generic-api-key]** `.secrets.baseline:230` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.5848296)
- **[gitleaks:generic-api-key]** `.secrets.baseline:236` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.853056)
- **[gitleaks:generic-api-key]** `.secrets.baseline:237` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.853056)
- **[gitleaks:generic-api-key]** `.secrets.baseline:245` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.5848296)
- **[gitleaks:generic-api-key]** `.secrets.baseline:246` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.853056)
- **[gitleaks:generic-api-key]** `.secrets.baseline:254` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.7775671)
- **[gitleaks:generic-api-key]** `.secrets.baseline:255` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.5848296)
- **[gitleaks:generic-api-key]** `.secrets.baseline:263` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.5848296)
- **[gitleaks:generic-api-key]** `.secrets.baseline:264` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.7775671)
- **[gitleaks:generic-api-key]** `.secrets.baseline:273` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.5848296)
- **[semgrep:generic.secrets.security.detected-aws-access-key-id-value.detected-aws-access-key-id-value]** `/home/kirky/projects/base/inklog/reviews/tiangang/gitleaks.json:708` — AWS Access Key ID Value detected. This is a sensitive credential and should not be hardcoded here. Instead, read this value from an environment variable or keep it in a separate, private file.
- **[semgrep:generic.secrets.security.detected-aws-access-key-id-value.detected-aws-access-key-id-value]** `/home/kirky/projects/base/inklog/reviews/tiangang/gitleaks.json:709` — AWS Access Key ID Value detected. This is a sensitive credential and should not be hardcoded here. Instead, read this value from an environment variable or keep it in a separate, private file.
- **[semgrep:generic.secrets.security.detected-aws-access-key-id-value.detected-aws-access-key-id-value]** `/home/kirky/projects/base/inklog/reviews/tiangang/gitleaks.json:748` — AWS Access Key ID Value detected. This is a sensitive credential and should not be hardcoded here. Instead, read this value from an environment variable or keep it in a separate, private file.
- **[semgrep:generic.secrets.security.detected-aws-access-key-id-value.detected-aws-access-key-id-value]** `/home/kirky/projects/base/inklog/reviews/tiangang/gitleaks.json:749` — AWS Access Key ID Value detected. This is a sensitive credential and should not be hardcoded here. Instead, read this value from an environment variable or keep it in a separate, private file.
- **[gitleaks:generic-api-key]** `docs/SECURITY.md:282` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.321928)
- **[gitleaks:generic-api-key]** `docs/USER_GUIDE.md:749` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.3908052)
- **[gitleaks:generic-api-key]** `examples/encryption.rs:13` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.3908052)
- **[gitleaks:generic-api-key]** `examples/encryption_compression.rs:23` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.3908052)
- **[gitleaks:generic-api-key]** `examples/production/microservice_logging.rs:234` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.3908052)
- **[gitleaks:generic-api-key]** `examples/src/bin/log_sanitizer.rs:60` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.523562)
- **[gitleaks:jwt]** `examples/src/bin/masking.rs:199` — Hardcoded secret detected by rule 'jwt' (entropy=5.4440703)
- **[gitleaks:jwt]** `examples/src/bin/masking.rs:202` — Hardcoded secret detected by rule 'jwt' (entropy=5.4440703)
- **[gitleaks:generic-api-key]** `examples/src/bin/masking.rs:215` — Hardcoded secret detected by rule 'generic-api-key' (entropy=5.080274)
- **[gitleaks:generic-api-key]** `examples/src/bin/masking.rs:218` — Hardcoded secret detected by rule 'generic-api-key' (entropy=5.080274)
- **[gitleaks:github-pat]** `examples/src/bin/masking.rs:244` — Hardcoded secret detected by rule 'github-pat' (entropy=4.421928)
- **[gitleaks:slack-bot-token]** `examples/src/bin/masking.rs:245` — Hardcoded secret detected by rule 'slack-bot-token' (entropy=4.141604)
- **[gitleaks:stripe-access-token]** `examples/src/bin/masking.rs:246` — Hardcoded secret detected by rule 'stripe-access-token' (entropy=4.521641)
- **[gitleaks:generic-api-key]** `examples/src/bin/masking.rs:248` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.5034165)
- **[gitleaks:generic-api-key]** `examples/src/bin/masking.rs:251` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.5034165)
- **[gitleaks:curl-auth-header]** `src/config.rs:1379` — Hardcoded secret detected by rule 'curl-auth-header' (entropy=3.3371754)
- **[gitleaks:curl-auth-header]** `src/config.rs:1455` — Hardcoded secret detected by rule 'curl-auth-header' (entropy=3.31764)
- **[gitleaks:jwt]** `src/masking.rs:614` — Hardcoded secret detected by rule 'jwt' (entropy=5.4440703)
- **[gitleaks:generic-api-key]** `src/masking.rs:631` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.9541965)
- **[gitleaks:generic-api-key]** `src/sink/encryption.rs:115` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.3908052)
- **[gitleaks:generic-api-key]** `src/sink/encryption.rs:126` — Hardcoded secret detected by rule 'generic-api-key' (entropy=5)
- **[gitleaks:generic-api-key]** `src/support/io/sink/file.rs:2331` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.875)
- **[gitleaks:generic-api-key]** `src/support/io/sink/file.rs:2364` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.875)
- **[gitleaks:generic-api-key]** `src/support/processing/masking.rs:1344` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.9068904)
- **[gitleaks:generic-api-key]** `src/support/processing/masking.rs:1352` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.321928)
- **[gitleaks:generic-api-key]** `src/validation/sanitize.rs:237` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.7004397)
- **[gitleaks:generic-api-key]** `tests/combinations/complex_features_test.rs:28` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.3908052)
- **[gitleaks:generic-api-key]** `tests/e2e_advanced.rs:1054` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.7254806)
- **[gitleaks:jwt]** `tests/e2e_advanced.rs:1064` — Hardcoded secret detected by rule 'jwt' (entropy=5.250351)
- **[gitleaks:jwt]** `tests/e2e_advanced.rs:1066` — Hardcoded secret detected by rule 'jwt' (entropy=5.250351)
- **[gitleaks:jwt]** `tests/e2e_advanced.rs:1472` — Hardcoded secret detected by rule 'jwt' (entropy=5.250351)
- **[gitleaks:generic-api-key]** `tests/e2e_advanced.rs:1493` — Hardcoded secret detected by rule 'generic-api-key' (entropy=3.640224)
- **[gitleaks:generic-api-key]** `tests/integration/cli/cli_test.rs:135` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.3908052)
- **[gitleaks:generic-api-key]** `tests/integration/comprehensive_validation_test.rs:32` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.3908052)
- **[gitleaks:jwt]** `tests/unit/masking_test.rs:117` — Hardcoded secret detected by rule 'jwt' (entropy=5.4440703)
- **[gitleaks:aws-access-token]** `tests/unit/masking_test.rs:132` — Hardcoded secret detected by rule 'aws-access-token' (entropy=4.0841837)
- **[gitleaks:aws-access-token]** `tests/unit/masking_test.rs:133` — Hardcoded secret detected by rule 'aws-access-token' (entropy=3.9841838)
- **[gitleaks:aws-access-token]** `tests/unit/masking_test.rs:134` — Hardcoded secret detected by rule 'aws-access-token' (entropy=4.0841837)
- **[gitleaks:generic-api-key]** `tests/unit/masking_test.rs:151` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.9541965)
- **[gitleaks:stripe-access-token]** `tests/unit/masking_test.rs:152` — Hardcoded secret detected by rule 'stripe-access-token' (entropy=4.2516294)
- **[gitleaks:generic-api-key]** `tests/unit/masking_test.rs:153` — Hardcoded secret detected by rule 'generic-api-key' (entropy=4.593069)
- **[gitleaks:github-pat]** `tests/unit/masking_test.rs:571` — Hardcoded secret detected by rule 'github-pat' (entropy=5.221928)
- **[gitleaks:slack-bot-token]** `tests/unit/masking_test.rs:586` — Hardcoded secret detected by rule 'slack-bot-token' (entropy=4.19716)
- **[gitleaks:stripe-access-token]** `tests/unit/masking_test.rs:596` — Hardcoded secret detected by rule 'stripe-access-token' (entropy=4.8397756)
- **[gitleaks:generic-api-key]** `tests/unit/masking_test.rs:610` — Hardcoded secret detected by rule 'generic-api-key' (entropy=5.2018414)
- **[gitleaks:private-key]** `tests/unit/masking_test.rs:624` — Hardcoded secret detected by rule 'private-key' (entropy=3.926589)
- **[trufflehog:Postgres]** `unknown file:?` — Secret detected by Postgres detector (unverified)
- **[trufflehog:Roaring]** `unknown file:?` — Secret detected by Roaring detector (unverified)
- **[trufflehog:URI]** `unknown file:?` — Secret detected by URI detector (unverified)
- **[trufflehog:FTP]** `unknown file:?` — Secret detected by FTP detector (unverified)
- **[trufflehog:Box]** `unknown file:?` — Secret detected by Box detector (unverified)
- **[trufflehog:EightxEight]** `unknown file:?` — Secret detected by EightxEight detector (unverified)

### Medium

- **[cargo-audit:RUSTSEC-2026-0235]** `Cargo.lock:?` — rkyv 0.7.46: Insufficient archive validation can cause out-of-bounds reads in archives containing Rc/Arc
