# Implementation Plan

## In Progress

- **conduit-release-workflow**
  - Goal: 通过 GitHub Actions 在版本 tag 推送时自动构建并发布 `conduit` 的 Linux/macOS amd64+arm64 可执行文件。
  - Scope: release workflow、矩阵构建、GitHub Release 资产上传，以及对应的 README 发布说明。
  - Detail Plan: `docs/plan/conduit-release-workflow.md`

- **conduit-cli-roadmap**
  - Goal: 将当前仅有 `expose` 的 CLI 演进为以 `connect` 为核心的完整 `conduit` 命令体系。
  - Scope: 命令分层、连接规范统一，以及先面向本地 daemon tunnel 的生命周期管理能力分阶段落地。
  - Detail Plan: `docs/plan/conduit-cli-roadmap.md`

- **conduit-expose**
  - Goal: 完成纯 Rust SSH 反向端口转发主链路，并作为未来 `conduit connect` 的运行时基础。
  - Scope: 前台运行、`--daemon` 后台运行、密码/私钥认证、远端默认公网绑定、错误路径与基础测试收口，以及 SSH 会话断开后的自动重连韧性与 `Reconnecting` 状态可见性。
  - Detail Plan: `docs/plan/conduit-expose.md`
