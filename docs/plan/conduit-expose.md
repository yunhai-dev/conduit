## Feature: conduit expose
**Status**: In Progress

> Note: 当前 `expose` 已视为未来 `conduit connect` 的 runtime 基础层。后续 CLI 重组优先在命令层兼容迁移，不重写已跑通的 SSH 反向转发主链路。
> Progress: `connect` 将复用同一条 `ExposeConfig -> expose::run` 运行链路，`expose` 暂时继续保留为兼容入口。

### Stages
1. **Stage 1**: CLI 与项目骨架
   - **Status**: Complete
   - **Validation**: `cargo check`，`cargo run -- expose --help`
2. **Stage 2**: 前台 SSH expose 主链路
   - **Status**: Code Complete, Manual Validation Pending
   - **Done**:
     - 已接通 SSH 建连与密码/私钥认证
     - 已注册远端 TCP reverse forward
     - 已接收 forwarded TCP channel 并转发到本地 TCP 服务
     - 已将字节转发桥接泛化为 `AsyncRead + AsyncWrite`
   - **Validation**: 本地启动测试服务后，通过远端端口访问到本地服务
3. **Stage 3**: `--daemon` 后台运行
   - **Status**: Complete
   - **Done**:
     - 前台/后台共用同一条 `expose::run` 运行链路
     - daemonize 已前移到 Tokio runtime 和 tracing 初始化之前，避免 fork 后复用已初始化运行时
     - daemon 启动时会先写入解析后的 log path，再创建 managed tunnel registry record
     - 已手工验证 `connect --daemon`、`list --all`、`status <id>`、`logs <id>` 路径
   - **Validation**: 后台启动成功，日志可见，生命周期命令可读取到运行中的 tunnel
4. **Stage 4**: 错误路径与基础测试收口
   - **Status**: In Progress
   - **Done**:
     - 已补充 `--server` 为 `host:port` / `[ipv6]:port` 的校验
     - 已补充 server/remote-host 基础单元测试
     - 已更新 `--server` 错误文案，避免误导为仅支持 IP socket address
   - **Validation**: `cargo test`，认证错误/端口冲突/本地服务未启动时错误信息清晰
5. **Stage 5**: Runtime 自动重连韧性
   - **Status**: In Progress
   - **Done**:
     - 计划在 `expose::run` 外层加入 reconnect supervisor，避免 SSH session 瞬断时前台/后台直接退出
     - 保持单条 forwarded 连接失败仅记录日志，不扩大为整条 tunnel 失败
     - daemon tunnel 在 SSH session 掉线后切换到 `Reconnecting`，待 remote forward 恢复后再回到 `Running`
   - **Validation**: `cargo test` / `cargo check`，并手工验证 SSH session 中断后状态切到 `Reconnecting` 且可以自动恢复转发；认证失败时应快速失败而不是无限重试
