# conduit

> Pure Rust SSH tunnel CLI.

`conduit` 用来通过 SSH 建立 reverse tunnel 和 local forward tunnel。当前推荐使用 `conduit connect` 处理反向隧道，使用 `conduit forward` 处理正向隧道；`conduit expose` 继续保留为兼容入口。除了前台运行外，也支持 `--daemon` 后台运行，以及对本地启动的 managed tunnel 做生命周期管理。

## Highlights

- 通过 `connect --remote <SPEC>` 建立 reverse TCP tunnel
- 通过 `forward -L <SPEC> -r <SERVER>` 建立 local forward tunnel
- `forward` 首版支持多条 `-L` 映射共用一条 SSH session
- 支持密码认证和私钥认证
- 支持前台运行与 `--daemon` 后台运行
- 支持本地 managed tunnel 生命周期命令：`list`、`status` / `show`、`logs`、`close`、`stop`、`restart`、`delete`
- 提供 `tunnel` 子命令别名层：`create`、`list`、`show` / `status`、`close`、`stop`、`restart`、`delete`、`logs`

## Quick Start

下面示例默认已经安装 `conduit`。如果你直接从源码运行，可以把 `conduit ...` 替换为 `cargo run -- ...`。

### Build locally

```bash
cargo build
cargo run -- --help
```

### Install locally

```bash
cargo install --path .
```

### Create a tunnel

```bash
conduit connect \
  --remote root@example.com:8080:3000 \
  --password "your-password" \
  --insecure-accept-host-key
```

### Run in background

```bash
conduit connect \
  --remote root@example.com:8080:3000 \
  --password "your-password" \
  --daemon \
  --insecure-accept-host-key
```

### Inspect a managed tunnel

```bash
conduit list
conduit status <ID>
conduit logs <ID> --tail 50
```

## Releases

- 推送匹配 `v*` 的 Git tag（例如 `v0.1.0`）后，GitHub Actions 会自动构建并上传 release 资产
- 当前自动发布的目标：
  - `x86_64-unknown-linux-musl`
  - `aarch64-unknown-linux-musl`
  - `x86_64-apple-darwin`
  - `aarch64-apple-darwin`
- 资产命名格式：`conduit-<tag>-<target>.tar.gz`
- 每个压缩包内包含：`conduit`、`README.md`

## Command Overview

```bash
conduit connect
conduit forward
conduit expose
conduit list
conduit status
conduit show
conduit close
conduit stop
conduit restart
conduit delete
conduit logs
conduit tunnel <subcommand>
```

Help:

```bash
conduit --help
conduit connect --help
conduit forward --help
conduit tunnel --help
```

## Usage

### `connect --remote <SPEC>`

`connect` 的核心参数格式：

```text
[USER@]SERVER:REMOTE_PORT:LOCAL_PORT
[USER@]SERVER:REMOTE_PORT:LOCAL_HOST:LOCAL_PORT
```

说明：

- `USER@` 可省略，也可以改用 `--user`
- `SERVER` 是 SSH server 主机名或 IP
- `REMOTE_PORT` 是远端暴露端口
- `LOCAL_PORT` 是本地服务端口
- 如果省略 `LOCAL_HOST`，默认使用 `127.0.0.1`
- SSH server 端口通过 `--port` 指定，默认 `22`

使用私钥认证：

```bash
conduit connect \
  --remote root@example.com:8080:3000 \
  --key ~/.ssh/id_ed25519 \
  --insecure-accept-host-key
```

### `connect --daemon`

如果未显式传入 `--log-file`，daemon tunnel 会自动分配日志文件，默认位于：

```text
~/.conduit/logs/<tunnel-id>.log
```

对应的本地 tunnel 元数据记录位于：

```text
~/.conduit/tunnels/
```

### `forward -L <SPEC> -r <SERVER>`

`forward` 用来建立本地端口转发，核心参数格式：

```text
-L LOCAL_PORT:TARGET_HOST:TARGET_PORT
-r [USER@]SERVER[:PORT]
```

说明：

- `-L` 可重复传入，多条映射共用同一条 SSH session
- `LOCAL_PORT` 是本地监听端口
- `TARGET_HOST:TARGET_PORT` 是通过 SSH server 侧访问的目标地址
- `-r` 中可直接带 `USER@`，也可以改用 `--user`
- 默认绑定 `127.0.0.1`；如需暴露到局域网，可使用 `--bind <ADDR>` 或 `--gateway`

示例：

```bash
conduit forward \
  -L 8080:web.internal:80 \
  -L 5432:db.internal:5432 \
  -r root@example.com:22 \
  --key ~/.ssh/id_ed25519 \
  --insecure-accept-host-key
```

后台运行：

```bash
conduit forward \
  -L 8080:web.internal:80 \
  -r root@example.com \
  --password "your-password" \
  --daemon \
  --insecure-accept-host-key
```

### Compatibility: `expose`

如果你仍想沿用旧参数风格：

```bash
conduit expose \
  --server example.com:22 \
  --user root \
  --local 127.0.0.1:3000 \
  --remote-port 8080 \
  --password "your-password" \
  --insecure-accept-host-key
```

参数含义：

- `--server`：SSH server 地址，格式 `host:port`
- `--local`：本地 TCP 目标，格式 `host:port`
- `--remote-port`：远端暴露端口
- `--remote-host`：远端绑定地址，默认 `0.0.0.0`

## Managed Tunnel Operations

### List / show / logs

```bash
conduit list
conduit status <ID>
conduit show <ID>
conduit logs <ID>
conduit logs <ID> --tail 50
conduit logs <ID> --follow
```

等价别名：

```bash
conduit tunnel list
conduit tunnel show <ID>
conduit tunnel status <ID>
conduit tunnel logs <ID>
```

### Close / stop / restart / delete

关闭单个 tunnel：

```bash
conduit close <ID>
conduit tunnel close <ID>
```

停止全部本地托管 tunnel：

```bash
conduit stop
conduit stop --all
conduit tunnel stop
```

重启单个 tunnel：

```bash
conduit restart <ID> \
  --password "your-password" \
  --insecure-accept-host-key
```

或：

```bash
conduit tunnel restart <ID> \
  --key ~/.ssh/id_ed25519 \
  --insecure-accept-host-key
```

注意：`restart` 会复用已记录的 tunnel 元数据，但不会复用认证信息，因此必须重新传入 `--password` 或 `--key`。

删除本地记录和关联日志：

```bash
conduit delete <ID>
conduit tunnel delete <ID>
```

如果目标 tunnel 进程仍在运行，需要显式使用 `--force`：

```bash
conduit delete <ID> --force
```

## Limitations

- 当前已支持 **reverse TCP tunnel** 与 **local forward tunnel**；`dynamic` / `monitor` / `stats` / `server` 等高级模式仍未实现
- `tunnel create` 当前仍对应 reverse/connect 路径，forward 暂未挂到 `tunnel` 子命令族
- V1 仍要求显式传入 `--insecure-accept-host-key`
- 生命周期命令当前只管理“由本地 CLI 启动、且以 daemon 模式运行”的 tunnel
- `restart` 不会持久化认证信息，执行时必须重新传入 `--password` 或 `--key`
- daemon 模式仅支持 Unix 平台

## Development

```bash
cargo fmt
cargo check
cargo clippy --all-targets --all-features
cargo test
cargo run -- --help
```

## Recommendations

- 新使用场景优先使用 `connect`
- `expose` 仅作为兼容入口保留
- 如果你需要后续管理 tunnel，启动时请使用 `--daemon`
- 如果你只想临时转发一次，前台运行即可
