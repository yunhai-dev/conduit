## Feature: conduit forward
**Status**: In Progress

### Goals
- 新增 `conduit forward`，支持 SSH 正向隧道（local forward）。
- 首版支持多条 `-L` 映射共用一条 SSH session。
- 尽量复用现有 SSH 认证、bridge、daemon、registry 与 reconnect 基础设施。

### Stages
1. **Stage 1**: CLI 与 forward config
   - **Status**: Complete
   - 新增 `conduit forward -L <SPEC> -r <SERVER>`
   - `-L` 支持重复传入，格式为 `LOCAL_PORT:TARGET_HOST:TARGET_PORT`
   - `-r` 解析 `[USER@]SERVER[:PORT]`
   - 复用现有 password/private-key/insecure-host-key 参数
   - **Validation**: `cargo test` 通过，`cargo run -- forward --help` 输出已覆盖新命令参数

2. **Stage 2**: Forward runtime 主链路
   - **Status**: In Progress
   - 已新增 forward runtime，建立本地 listener 并为每个连接打开 SSH `direct-tcpip`
   - 已复用现有 `bridge(...)` 做双向转发
   - **Validation**: 代码已接入并通过编译/测试；待真实 SSH 环境下完成单条 `-L` 手工验证

3. **Stage 3**: 多 `-L` 与自动重连
   - **Status**: In Progress
   - 已支持多条 `-L` 共用一条 SSH session
   - 已接入 SSH session 掉线自动重连，并在 daemon 模式下写入 `Reconnecting` 状态
   - 单条连接失败仅记录日志，不扩大为整条 tunnel 失败
   - **Validation**: 解析与状态路径已通过测试；待真实 SSH 环境下完成多条 `-L` 与断线恢复验证

4. **Stage 4**: daemon / registry / restart 集成
   - **Status**: In Progress
   - `forward --daemon` 已接入现有托管 tunnel 生命周期
   - registry 已持久化 forward 元数据以支持 `list/status/logs/restart`
   - **Validation**: 编译、单元测试与 `forward --help` 已通过；待真实 daemon 流程手工验证

5. **Stage 5**: 文档同步与收尾
   - **Status**: In Progress
   - 更新 `README.md` 与 `docs/plan/conduit-cli-roadmap.md`
   - 对齐帮助文本、示例与实现边界
   - **Validation**: README 示例需与最新 `--help` 输出保持一致
