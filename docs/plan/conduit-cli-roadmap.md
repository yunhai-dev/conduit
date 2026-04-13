## Feature: conduit CLI roadmap
**Status**: In Progress

### Goals
- 从当前仅支持 `conduit expose` 的脚手架，演进到以 `conduit connect` 为核心的完整命令体系。
- 保持实现分段推进：先收敛现有 reverse tunnel 主链路，再做命令重组，再扩展隧道管理和观测能力。
- 避免一次性铺开全部命令；优先落地能复用当前 runtime 的最小闭环。

### Command strategy
1. **Phase A: Stabilize current runtime**
   - 保留 `expose`，完成其手工验证、daemon 验证、错误路径收口。
   - 将现有 runtime 明确视为未来 `connect` 的底层实现基础。
   - **Validation**: `cargo test`，前台/后台手工验证通过。

2. **Phase B: Introduce `connect` as the primary public command**
   - **Status**: In Progress
   - 新增 `conduit connect`，语义上替代 `conduit expose`。
   - 第一阶段不要一次引入完整大参数面；先支持一条最小兼容路径：
     - `--remote <SPEC>` 采用当前已选格式：`[USER@]SERVER:REMOTE_PORT:LOCAL_PORT` 或 `[USER@]SERVER:REMOTE_PORT:LOCAL_HOST:LOCAL_PORT`
     - 通过 `--port` 单独指定 SSH server port，默认 `22`
     - 继续复用当前 password/private-key/insecure-host-key/runtime daemon 能力
   - `expose` 在过渡期保留为兼容命令。
   - **Validation**: `conduit connect --help`、旧 `expose` 路径仍可工作。

3. **Phase C: Add lifecycle and inspection commands**
   - **Status**: In Progress
   - 先实现最小价值、最能闭环的命令，而不是一次铺满：
     - `list`
     - `status(show)`
     - `close`
     - `stop`
     - `logs`
     - `tunnel`（先作为别名层，当前先落地 `create` / `list` / `show(status)` / `close` / `stop` / `restart` / `delete` / `logs`）
   - 当前范围收敛为：仅管理由本地 CLI 启动、且以 daemon 模式运行的 tunnel。
   - `tunnel restart` / 顶层 `restart` 当前采用最小闭环：复用已记录的 tunnel 元数据，但认证信息不落盘，重启时需重新传入 `--password` 或 `--key`。
   - `tunnel delete` / 顶层 `delete` 当前删除本地 registry 和关联日志；若目标进程仍在运行，需显式传入 `--force` 才会先终止再删除。
   - 这些命令依赖本地状态持久化或 daemon/runtime registry，需要先定义隧道记录模型。
   - **Validation**: 可查看、定位、关闭本地托管的 daemon tunnel，并能读取其日志；也可批量停止本地托管 tunnel。

4. **Phase D: Add config-centered workflows**
   - 实现：
     - `init`
     - `config show/get/set/validate`
     - `test`
     - `completion`
   - 引入配置文件后，再统一全局选项如 `--config` / `--env` / `-v` / `-q`。
   - **Validation**: 配置初始化、读取、覆盖关系清晰。

5. **Phase E: Expand remaining advanced tunnel modes**
   - **Status**: In Progress
   - `forward` 已以顶层 `conduit forward` 落地，首版支持多条 `-L` 映射共用一条 SSH session，并复用 daemon / registry / restart 链路。
   - 在 reverse TCP 与 local forward 路径稳定后，再评估：
     - `dynamic`
     - `tunnel` 子命令族的进一步扩展
     - `monitor` / `stats`
     - `server`
   - 这些功能仍然意味着新的 runtime 边界，不适合和当前 connect / forward 主链路收口并行硬上。
   - **Validation**: 每个命令独立有最小可用闭环。

### Decisions
- `connect` 将作为未来主命令；`expose` 是当前实现阶段名称，不适合作为长期唯一入口。
- 你给出的命令规划可作为目标蓝图，但不应一次性全部实现。
- 下一次真正的代码改造重点应是：
  1. 定义 `RemoteSpec`
  2. 新增 `connect` 命令并复用现有 expose runtime
  3. 决定 `expose` 是保留别名还是逐步隐藏

### Near-term implementation order
1. 完成 `conduit expose` 当前 Stage 2/3/4 的手工验证和问题修正。
2. 设计并实现 `connect --remote <SPEC>` 的解析模型。
3. 将 CLI 顶层从单一 `expose` 扩展为 `connect` + `expose`（兼容）。
4. 再进入 tunnel registry / list / status / close 的设计。

### Out of scope for the next change
- 不在下一步里一次引入 `server` / `dynamic` / `monitor` / `stats` / `config` 全量子命令。
- 不在下一步里实现所有高级参数，如负载均衡、健康检查、TLS、限速、压缩、MFA。
- 不在没有本地状态模型之前实现 `list/status/close/stop`。
