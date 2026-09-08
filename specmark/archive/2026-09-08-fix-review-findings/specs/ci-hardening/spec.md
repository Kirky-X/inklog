# Spec — ci-hardening

> Delta spec for change `fix-review-findings`. 覆盖测试容器加固与 GitHub Actions 供应链加固需求。

## Requirements

### R-ci-hardening-001: 测试容器安全加固

`docker/docker-compose.test.yml` 的 postgres、mysql、redis 三个服务各自具备：

- `security_opt: ["no-new-privileges:true"]`
- `read_only: true`
- 覆盖镜像必需可写路径的 `tmpfs` 挂载（postgres：`/var/lib/postgresql/data`、`/var/run/postgresql`、`/tmp`；mysql：`/var/lib/mysql`、`/var/run/mysqld`、`/tmp`；redis：`/data`、`/tmp`）

**验收标准：**
- `docker compose -f docker/docker-compose.test.yml config` 语法解析通过
- yml 中三服务均含上述键；无服务以可写根文件系统运行

### R-ci-hardening-002: Actions 可变标签全部 pin 到 commit SHA

`.github/workflows/*.yml` 中所有 `uses: owner/repo@<ref>` 的 `<ref>` 替换为经 `git ls-remote` 解析的完整 commit SHA（annotated tag 取 `^{}` 剥离值），行尾保留 `# <原ref>` 注释。

**验收标准：**
- `grep -rn "uses:" .github/workflows/` 总数不变（45 处），且不存在 `@[v0-9]` 或 `@master`/`@main`/`@stable` 形式的可变引用
- 每个 SHA 可经 `git ls-remote` 对应回原 tag/分支指向的 commit（解析记录留存于提交说明或脚本输出）

## Constraints

- 不改动工作流的任务逻辑、触发条件与 feature 列表
- SHA pin 不改变执行的 action 版本语义（解析自当前 tag/分支指向的 commit）

## Out of Scope

- Dependabot/renovate 自动化 SHA 升级；workflow 权限最小化（`permissions:` 收紧）
