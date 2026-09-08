# Spec — http-ip-whitelist

> Main spec for capability `http-ip-whitelist`.

## Requirements

### R-http-ip-whitelist-001: 配置期白名单条目格式校验（fail-fast）

`InklogConfig::validate` 的 HTTP 段对 `ip_whitelist` 每条目校验：必须能解析为 `std::net::IpAddr` 或 `ipnet::IpNet`（与 `http_server` 运行期解析语义一致）；不合法时返回 `InklogError`，错误消息含条目原文与序号。

**验收标准：**
- 含 `"10.0.0.1"`、`"192.168.1.0/24"`、`"2001:db8::1"` 的配置通过 `validate`
- 含 `"10.0.0.0/33"` 或 `"not-an-ip"` 的配置 `validate` 返回 `Err`，消息含对应条目原文
- 既有合法配置测试（`cargo test -p inklog --lib domain::config`）全部通过

### R-http-ip-whitelist-002: 运行期解析失败一次性告警

`http_server.rs` 白名单匹配中条目解析失败时，经进程级原子标志仅首次输出 `tracing::warn!`（含条目原文），后续同条目或同进程内不再重复告警；解析失败的条目不匹配任何来源 IP（fail-closed 语义与现状一致）。

**验收标准：**
- 新增测试：白名单含非法条目时，合法白名单条目的放行/拒绝行为与修复前一致（既有白名单测试全部通过）
- `grep -n "warn" src/domain/core/http_server.rs` 在解析失败分支命中告警调用，且告警受进程级标志守卫（非每请求执行）

## Constraints

- 不改变白名单匹配的放行/拒绝判定语义（仅新增可观测性）
- 运行期告警不得引入每请求锁竞争（用原子标志，不用 Mutex/log 去重表）

## Out of Scope

- CIDR 语义增强（如 IPv6 zone）；白名单热更新；鉴权 token 逻辑
