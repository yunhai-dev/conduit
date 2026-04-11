# Conduit 完整命令规划

## 一、命令体系架构

```
conduit
├── connect     # 建立隧道连接（核心）
├── server      # 启动服务端
├── forward     # 本地端口转发
├── dynamic     # 动态SOCKS代理
├── tunnel      # 隧道管理
├── list        # 查看隧道列表
├── close       # 关闭隧道
├── stop        # 停止所有隧道
├── status      # 查看状态
├── monitor     # 监控流量
├── stats       # 统计信息
├── config      # 配置管理
├── init        # 初始化配置
├── test        # 测试连接
├── logs        # 查看日志
├── version     # 版本信息
├── help        # 帮助信息
└── completion  # 命令补全
```

## 二、详细命令规范

### 1. `conduit connect` - 建立反向隧道连接

```bash
# 基本语法
conduit connect [OPTIONS] --remote <SPEC>

# 参数说明
-r, --remote <SPEC>           远程端口映射规范，格式: [USER@]SERVER:REMOTE_PORT:LOCAL_HOST:LOCAL_PORT
                               示例: root@example.com:8080:localhost:3000
                               示例: example.com:8080:3000 (本地主机默认为localhost)

# 认证选项
    --user <USER>              SSH用户名
    --key <PATH>               SSH私钥路径 (默认: ~/.ssh/id_rsa)
    --password                 提示输入密码
    --cert <PATH>              客户端证书路径
    --token <TOKEN>            访问令牌
    --mfa                      启用双因素认证

# 连接选项
    --port <PORT>              SSH服务端端口 (默认: 22)
    --timeout <SECONDS>        连接超时 (默认: 30)
    --keep-alive <SECONDS>     保活间隔 (默认: 60)
    --retry <COUNT>            重试次数 (默认: 3)
    --retry-interval <SECONDS> 重试间隔 (默认: 5)

# 隧道选项
    --protocol <PROTO>         协议类型: tcp, udp, http, https, socks5 (默认: tcp)
    --encrypt <ALGO>           加密算法: aes128-gcm, aes256-gcm, chacha20 (默认: aes256-gcm)
    --compression <ALGO>       压缩算法: none, zstd, gzip, lz4 (默认: none)
    --compress-level <LEVEL>   压缩级别: 1-22 (zstd), 1-9 (gzip)
    --chunk-size <SIZE>        数据块大小: 16kb, 64kb, 256kb, 1mb (默认: 64kb)
    --pipeline <N>             管道并发数 (默认: 10)

# 流量控制
    --rate-limit <SPEC>        限速，格式: UPLOAD/DOWNLOAD，单位: kb, mb, gb
                               示例: 10mb/5mb, 1gb/500mb
    --max-conn <N>             最大并发连接数 (默认: 0 = 无限制)
    --buffer-size <SIZE>       缓冲区大小 (默认: 32kb)

# 调度选项
    --daemon                   后台运行模式
    --pid-file <PATH>          PID文件路径 (默认: /var/run/conduit.pid)
    --schedule <CRON>          定时调度，格式: CRON表达式
                               示例: "0 9-17 * * 1-5" (工作日9-17点)
    --on-demand                按需启动（监听第一个连接时建立隧道）
    --warmup <SECONDS>         预热时间（按需模式首次连接延迟）

# 高级功能
    --fallback <SPEC>          备用服务器，格式同--remote
    --load-balance <ALGO>      负载均衡算法: round-robin, least-conn, random, consistent-hash
    --backends <LIST>          多后端服务器，格式: host1:port1,host2:port2
    --health-check <PATH>      健康检查端点（HTTP/HTTPS）
    --health-interval <SECONDS> 健康检查间隔 (默认: 30)

# 安全选项
    --allow-ips <LIST>         IP白名单，格式: 192.168.1.0/24,10.0.0.1
    --deny-ips <LIST>          IP黑名单
    --tls-cert <PATH>          TLS证书路径
    --tls-key <PATH>           TLS私钥路径
    --verify-hostname          验证主机名

# 监控选项
    --metrics <ADDR>           Prometheus指标端点，格式: host:port
    --metrics-path <PATH>      指标路径 (默认: /metrics)
    --verbose, -v              详细输出
    --quiet, -q                静默模式
    --log-level <LEVEL>        日志级别: trace, debug, info, warn, error (默认: info)
    --log-file <PATH>          日志文件路径

# 性能优化
    --cache <SIZE>             缓存大小，单位: mb, gb
    --cache-ttl <SECONDS>      缓存TTL (默认: 300)
    --tcp-nodelay              启用TCP_NODELAY
    --tcp-keepalive            启用TCP_KEEPALIVE
    --tcp-fastopen             启用TCP_FASTOPEN

# 其他
    --config <PATH>            配置文件路径 (默认: ~/.config/conduit/config.toml)
    --env <NAME>               环境名称 (默认: default)

# 示例
conduit connect -r example.com:8080:3000
conduit connect -r root@example.com:2222:192.168.1.100:8080 --key ~/.ssh/id_ed25519
conduit connect -r example.com:8080:3000 -r example.com:8081:5432 --keep-alive 30
conduit connect -r example.com:8080:3000 --rate-limit 10mb/5mb --max-conn 100 --daemon
conduit connect -r primary.com:8080:3000 --fallback secondary.com:8080:3000 --load-balance least-conn
```

### 2. `conduit server` - 启动服务端

```bash
# 基本语法
conduit server [OPTIONS]

# 监听选项
    --bind <ADDR>               绑定地址 (默认: 0.0.0.0)
    --port <PORT>               监听端口 (默认: 22)
    --socket <PATH>             Unix socket路径
    --tls                       启用TLS
    --tls-cert <PATH>           TLS证书
    --tls-key <PATH>            TLS私钥

# SSH选项
    --host-key <PATH>           SSH主机密钥路径 (默认: /etc/ssh/ssh_host_rsa_key)
    --allowed-users <LIST>      允许的用户列表，格式: user1,user2
    --auth-methods <LIST>       认证方式: password, publickey, keyboard-interactive
    --max-auth-tries <N>        最大认证尝试次数 (默认: 6)
    --auth-timeout <SECONDS>    认证超时 (默认: 120)

# 隧道管理
    --max-tunnels <N>           最大隧道数 (默认: 100)
    --max-connections <N>       单隧道最大连接数 (默认: 1000)
    --idle-timeout <SECONDS>    空闲超时 (默认: 300)
    --keepalive-interval <SECONDS> 保活间隔 (默认: 60)

# 权限控制
    --allow-tcp-forwarding      允许TCP转发 (默认: true)
    --allow-stream-local-forward 允许本地流转发
    --permit-listen <PORT_RANGE> 允许监听的端口范围，格式: 1000-2000,3000-4000
    --deny-listen <PORT_RANGE>   禁止监听的端口范围

# 限流配置
    --global-rate-limit <SPEC>  全局限速，格式: UPLOAD/DOWNLOAD
    --per-user-rate-limit <SPEC> 每用户限速
    --per-tunnel-rate-limit <SPEC> 每隧道限速

# Web管理界面
    --web-admin <ADDR>          Web管理界面地址，格式: host:port
    --web-admin-auth            Web管理界面认证
    --web-admin-username <NAME> Web管理用户名
    --web-admin-password <PWD>  Web管理密码

# 监控选项
    --metrics <ADDR>            Prometheus指标端点
    --metrics-path <PATH>       指标路径 (默认: /metrics)

# 日志选项
    --log-level <LEVEL>         日志级别
    --log-format <FORMAT>       日志格式: json, text (默认: text)
    --log-file <PATH>           日志文件
    --access-log <PATH>         访问日志

# 其他
    --daemon                    后台运行
    --pid-file <PATH>           PID文件
    --config <PATH>             配置文件

# 示例
conduit server --port 2222 --host-key ~/.ssh/conduit_host_key
conduit server --bind 127.0.0.1 --port 2222 --allowed-users alice,bob
conduit server --web-admin :8080 --web-admin-username admin --metrics :9090
```

### 3. `conduit forward` - 本地端口转发

```bash
# 基本语法
conduit forward [OPTIONS] -L <SPEC> -r <SERVER>

# 参数说明
-L, --local <SPEC>             本地转发规范，格式: LOCAL_PORT:TARGET_HOST:TARGET_PORT
                               示例: 8080:internal-server:80
-r, --remote <SERVER>          远程服务器，格式: [USER@]SERVER[:PORT]

# 选项
    --bind <ADDR>               本地绑定地址 (默认: 127.0.0.1)
    --gateway                   网关模式（允许外部连接）
    --user <USER>               SSH用户
    --key <PATH>                私钥路径
    --keep-alive <SECONDS>      保活间隔
    --daemon                    后台运行

# 示例
conduit forward -L 8080:web:80 -r jump.example.com
conduit forward -L 3306:db:3306 -L 6379:redis:6379 -r gateway.com --gateway
```

### 4. `conduit dynamic` - 动态SOCKS5代理

```bash
# 基本语法
conduit dynamic [OPTIONS] -D <PORT> -r <SERVER>

# 参数说明
-D, --socks-port <PORT>        SOCKS5代理端口
-r, --remote <SERVER>          远程服务器

# 选项
    --bind <ADDR>               绑定地址 (默认: 127.0.0.1)
    --auth                      启用SOCKS5认证
    --username <NAME>           SOCKS用户名
    --password <PWD>            SOCKS密码
    --udp                       支持UDP关联
    --ipv6                      支持IPv6
    --daemon                    后台运行

# 示例
conduit dynamic -D 1080 -r proxy.example.com
conduit dynamic -D 1080 -r user@gateway.com:2222 --bind 0.0.0.0 --auth --username socksuser
```

### 5. `conduit tunnel` - 隧道管理子命令

```bash
# 创建隧道（connect的别名）
conduit tunnel create [OPTIONS] -r <SPEC>

# 列出隧道
conduit tunnel list [OPTIONS]
    --filter <FILTER>           过滤条件: running, stopped, failed, all
    --format <FORMAT>           输出格式: table, json, yaml (默认: table)

# 查看隧道详情
conduit tunnel show <TUNNEL_ID|NAME>
    --verbose, -v               显示详细信息
    --stats                     显示统计信息

# 修改隧道
conduit tunnel update <TUNNEL_ID|NAME> [OPTIONS]
    --rate-limit <SPEC>         更新限速
    --max-conn <N>              更新最大连接数
    --enabled <BOOL>            启用/禁用

# 重启隧道
conduit tunnel restart <TUNNEL_ID|NAME>

# 删除隧道
conduit tunnel delete <TUNNEL_ID|NAME> [--force]

# 导出隧道配置
conduit tunnel export <TUNNEL_ID|NAME> [--format toml|yaml|json]

# 示例
conduit tunnel list --format json --filter running
conduit tunnel show tunnel-abc123 --stats
conduit tunnel update web-tunnel --rate-limit 20mb/10mb
```

### 6. `conduit list` - 查看隧道列表

```bash
# 基本语法
conduit list [OPTIONS]

# 选项
    --all, -a                   显示所有隧道（包括已停止）
    --running                   只显示运行中的
    --failed                    只显示失败的
    --format <FORMAT>           输出格式: table, json, yaml, csv
    --watch, -w                 实时监控模式
    --interval <SECONDS>        刷新间隔 (默认: 2)

# 示例
conduit list --running --format table
conduit list --watch --interval 1
```

### 7. `conduit close` - 关闭隧道

```bash
# 基本语法
conduit close <TUNNEL_ID|NAME> [OPTIONS]

# 选项
    --force, -f                 强制关闭
    --graceful                  优雅关闭（等待现有连接完成）
    --timeout <SECONDS>         优雅关闭超时 (默认: 30)

# 示例
conduit close tunnel-abc123
conduit close web-tunnel --graceful --timeout 60
```

### 8. `conduit stop` - 停止所有隧道

```bash
# 基本语法
conduit stop [OPTIONS]

# 选项
    --all                       停止所有隧道（包括守护进程）
    --force                     强制停止
    --graceful                  优雅停止

# 示例
conduit stop --graceful
conduit stop --all --force
```

### 9. `conduit status` - 查看状态

```bash
# 基本语法
conduit status [OPTIONS]

# 选项
    --tunnel <ID|NAME>          指定隧道
    --detailed, -d              详细状态
    --health                    健康检查状态
    --format <FORMAT>           输出格式

# 示例
conduit status --detailed
conduit status --tunnel web-tunnel --health
```

### 10. `conduit monitor` - 监控流量

```bash
# 基本语法
conduit monitor [OPTIONS]

# 选项
    --tunnel <ID|NAME>          监控指定隧道
    --interval <SECONDS>        刷新间隔 (默认: 1)
    --format <FORMAT>           显示格式: table, graph, json
    --bandwidth                 显示带宽
    --connections               显示连接数
    --latency                   显示延迟
    --export <PATH>             导出监控数据
    --alert-threshold <SPEC>    告警阈值，格式: METRIC:VALUE
                               示例: bandwidth:100mb, errors:10

# 示例
conduit monitor --tunnel web-tunnel --bandwidth --connections
conduit monitor --interval 0.5 --format graph
conduit monitor --alert-threshold bandwidth:50mb --alert-threshold errors:5
```

### 11. `conduit stats` - 统计信息

```bash
# 基本语法
conduit stats [OPTIONS]

# 子命令
conduit stats summary              # 总览统计
conduit stats traffic              # 流量统计
conduit stats connections          # 连接统计
conduit stats errors               # 错误统计
conduit stats history              # 历史统计

# 选项
    --tunnel <ID|NAME>            指定隧道
    --period <PERIOD>             时间周期: hour, day, week, month, year
    --start <TIMESTAMP>           开始时间
    --end <TIMESTAMP>             结束时间
    --format <FORMAT>             输出格式
    --json                        输出JSON格式

# 示例
conduit stats traffic --period day --format table
conduit stats summary --json
conduit stats history --tunnel web-tunnel --start "2024-01-01" --end "2024-01-31"
```

### 12. `conduit config` - 配置管理

```bash
# 子命令
conduit config show [OPTIONS]               # 显示当前配置
conduit config set <KEY> <VALUE>            # 设置配置项
conduit config get <KEY>                    # 获取配置项
conduit config unset <KEY>                  # 删除配置项
conduit config list                         # 列出所有配置
conduit config edit                         # 编辑配置文件
conduit config validate                     # 验证配置文件
conduit config import <FILE>                # 导入配置
conduit config export [OPTIONS]             # 导出配置
conduit config profile <ACTION>             # 配置文件管理

# profile子命令
conduit config profile list                 # 列出所有配置文件
conduit config profile create <NAME>        # 创建配置文件
conduit config profile switch <NAME>        # 切换配置文件
conduit config profile delete <NAME>        # 删除配置文件

# 选项
    --env <NAME>                           环境名称
    --format <FORMAT>                      输出格式: toml, yaml, json
    --merge                                合并配置（导入时）

# 示例
conduit config set server.default_port 2222
conduit config profile create production
conduit config export --format yaml > backup.yaml
```

### 13. `conduit init` - 初始化配置

```bash
# 基本语法
conduit init [OPTIONS]

# 选项
    --interactive, -i          交互式初始化
    --quick                    快速初始化（使用默认值）
    --server <SERVER>          默认服务器地址
    --user <USER>              默认用户名
    --key <PATH>               默认私钥路径
    --force                    覆盖现有配置
    --template <NAME>          使用配置模板: minimal, standard, advanced, enterprise

# 示例
conduit init --interactive
conduit init --quick --server tunnel.example.com --user deploy
conduit init --template enterprise --force
```

### 14. `conduit test` - 测试连接

```bash
# 基本语法
conduit test [OPTIONS] -r <SPEC>

# 选项
    --bandwidth                测试带宽
    --latency                  测试延迟
    --packet-loss              测试丢包率
    --duration <SECONDS>       测试持续时间 (默认: 10)
    --parallel <N>             并发连接数 (默认: 1)
    --verbose, -v              详细输出

# 示例
conduit test -r example.com:8080:3000 --bandwidth --latency
conduit test -r example.com:8080:3000 --duration 30 --parallel 4
```

### 15. `conduit logs` - 查看日志

```bash
# 基本语法
conduit logs [OPTIONS]

# 选项
    --tunnel <ID|NAME>         指定隧道日志
    --follow, -f               实时跟踪
    --tail <N>                 显示最后N行 (默认: 100)
    --since <TIME>             显示之后的时间，格式: 2024-01-01T00:00:00, 1h, 30m
    --until <TIME>             显示之前的时间
    --level <LEVEL>            过滤日志级别: trace, debug, info, warn, error
    --grep <PATTERN>           过滤关键词
    --format <FORMAT>          输出格式: text, json

# 示例
conduit logs --tunnel web-tunnel --follow
conduit logs --since "1h" --level error --grep "connection failed"
```

### 16. `conduit version` - 版本信息

```bash
# 基本语法
conduit version [OPTIONS]

# 选项
    --verbose, -v              显示详细信息（依赖版本、构建信息）
    --format <FORMAT>          输出格式: text, json

# 示例
conduit version
conduit version --verbose
```

### 17. `conduit help` - 帮助信息

```bash
# 基本语法
conduit help [COMMAND]

# 示例
conduit help
conduit help connect
conduit help tunnel create
```

### 18. `conduit completion` - 命令补全

```bash
# 基本语法
conduit completion <SHELL>

# 支持的shell
    bash       生成bash补全脚本
    zsh        生成zsh补全脚本
    fish       生成fish补全脚本
    powershell 生成powershell补全脚本

# 选项
    --install  安装补全脚本

# 示例
conduit completion bash > /etc/bash_completion.d/conduit
conduit completion zsh --install
```

## 三、全局选项

所有命令都支持以下全局选项：

```bash
    --config <PATH>            指定配置文件路径
    --env <NAME>               指定环境名称
    --verbose, -v              增加详细程度（可多次使用: -v, -vv, -vvv）
    --quiet, -q                减少输出
    --color <WHEN>             彩色输出: auto, always, never (默认: auto)
    --no-color                 禁用彩色输出
    --timeout <SECONDS>        全局超时 (默认: 30)
```

## 四、配置文件示例

```toml
# ~/.config/conduit/config.toml

[default]
server = "tunnel.example.com"
port = 22
user = "deploy"
key_path = "~/.ssh/id_rsa"
keep_alive = 60
retry = 3

[default.tunnels.web]
remote = "example.com:8080:localhost:3000"
rate_limit = "10mb/5mb"
max_conn = 100
compression = "zstd"
encrypt = "aes256-gcm"

[default.tunnels.db]
remote = "example.com:5432:localhost:5432"
rate_limit = "50mb/50mb"

[production]
server = "prod-tunnel.company.com"
port = 2222
user = "prod-deploy"
key_path = "~/.ssh/prod_key"

[logging]
level = "info"
format = "json"
file = "/var/log/conduit.log"

[monitoring]
metrics = ":9090"
health_check = true
```

## 五、退出状态码

| 状态码 | 含义 |
|--------|------|
| 0 | 成功 |
| 1 | 一般错误 |
| 2 | 参数错误 |
| 3 | 配置错误 |
| 4 | 认证失败 |
| 5 | 连接失败 |
| 6 | 超时 |
| 7 | 权限拒绝 |
| 8 | 隧道已存在 |
| 9 | 隧道不存在 |
| 10 | 资源不足 |
| 99 | 未知错误 |

## 六、环境变量

```bash
CONDUIT_CONFIG        配置文件路径
CONDUIT_ENV           当前环境
CONDUIT_LOG_LEVEL     日志级别
CONDUIT_NO_COLOR      禁用颜色
CONDUIT_DEBUG         调试模式
CONDUIT_INSECURE      跳过证书验证
```

## 七、使用示例大全

```bash
# 快速开始
conduit init --quick --server myvps.com
conduit connect -r myvps.com:8080:3000

# 生产环境部署
conduit connect \
  -r api.example.com:8080:app:3000 \
  -r api.example.com:8081:mysql:3306 \
  --keep-alive 30 \
  --rate-limit 100mb/50mb \
  --max-conn 1000 \
  --compression zstd \
  --encrypt aes256-gcm \
  --daemon \
  --pid-file /var/run/conduit-api.pid \
  --log-file /var/log/conduit-api.log

# 高可用配置
conduit connect \
  -r primary.com:8080:3000 \
  --fallback secondary.com:8080:3000 \
  --load-balance least-conn \
  --health-check /health \
  --health-interval 10

# 开发环境
conduit forward -L 8080:localhost:3000 -L 5432:localhost:5432 -r dev-gateway.com
conduit dynamic -D 1080 -r dev-gateway.com --daemon

# 监控调试
conduit monitor --tunnel web-tunnel --interval 0.5 --format graph
conduit stats traffic --period hour --format json
conduit logs --tunnel web-tunnel --follow --level debug

# 批量操作
conduit config import prod-tunnels.yaml
conduit tunnel create -r prod.com:8080:3000 --name web
conduit tunnel create -r prod.com:8081:5432 --name db
conduit tunnel list --running

# 自动化脚本
#!/bin/bash
if conduit test -r myvps.com:8080:3000 --latency; then
    conduit connect -r myvps.com:8080:3000 --daemon
else
    echo "Connection test failed"
    exit 1
fi
```

## 八、命令别名速查

| 完整命令 | 短别名 |
|----------|--------|
| `conduit connect` | `conduit c` |
| `conduit list` | `conduit ls` |
| `conduit status` | `conduit st` |
| `conduit monitor` | `conduit mon` |
| `conduit stats` | `conduit sts` |
| `conduit config` | `conduit cfg` |
| `conduit version` | `conduit v` |
| `conduit help` | `conduit h` |

---

这套完整的命令规划将项目名称从 `sshnap` 改为 `conduit`，保持了所有功能的一致性和完整性。`conduit` 这个名字更好地体现了"通道/导管"的概念，简洁有力，适合作为内网穿透工具的名称。
