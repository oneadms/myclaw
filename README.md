<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.91+-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/platform-Linux%20x86__64-blue" alt="Platform">
  <img src="https://img.shields.io/github/v/release/oneadms/myclaw?color=green" alt="Release">
  <img src="https://img.shields.io/github/license/oneadms/myclaw" alt="License">
</p>

<h1 align="center">MyClaw</h1>

<p align="center">
  轻量级 OpenClaw 频道服务器 & 终端聊天客户端<br>
  <sub>Rust · async · WebSocket · TUI</sub>
</p>

---

## 它是什么

MyClaw 是一个基于 Rust 的分布式聊天系统，由五个 crate 组成：

```
用户(公网)                        云服务器                              Mac(内网)

myclaw-client ──WS──► myclaw-server ──WS──► myclaw-relay ◄──WS── myclaw-agent ──WS──► OpenClaw Gateway
   :TUI               :9800                :19000  :19001                              :18789
```

- **myclaw-server** — 频道节点服务器，桥接客户端与 Gateway，管理会话路由
- **myclaw-client** — 终端 TUI 聊天界面，支持流式响应
- **myclaw-relay** — 中继层，部署在云服务器，透明转发 server 与 agent 之间的消息
- **myclaw-agent** — 隧道代理，部署在内网 Mac，主动出站连接 relay 并桥接本地 Gateway
- **myclaw-common** — 共享协议定义与错误类型

## 特性

- 全异步架构，基于 tokio
- WebSocket 双向通信，支持流式消息推送
- Relay 中继层，支持 NAT 穿透（内网 Gateway 无需公网 IP）
- Agent 隧道代理，主动出站连接 + 断线自动重连（指数退避）
- Gateway 断线自动重连（指数退避）
- 心跳保活机制
- 基于 ratatui 的终端 UI，彩色消息展示
- TOML 配置文件，开箱即用

---

## 快速开始

### 从 Release 下载

```bash
# 下载 Linux x86_64 预编译包
curl -LO https://github.com/oneadms/myclaw/releases/download/v0.1.0/myclaw-linux-x86_64.tar.gz
tar xzf myclaw-linux-x86_64.tar.gz

# 启动服务器（云服务器）
./myclaw-relay -c config/relay.toml
./myclaw-server -c config/server.toml

# 启动代理（内网 Mac）
./myclaw-agent -c config/agent.toml

# 启动客户端
./myclaw-client -c config/client.toml
```

### 从源码构建

```bash
git clone https://github.com/oneadms/myclaw.git
cd myclaw
cargo build --release

# 产物在 target/release/ 下
```

---

## 配置

### 服务器 `config/server.toml`

```toml
[server]
host = "127.0.0.1"
port = 9800

[gateway]
url = "ws://127.0.0.1:19000"
node_id = "myclaw-node-01"
heartbeat_interval_secs = 30
reconnect_base_ms = 1000
reconnect_max_ms = 30000
```

| 字段 | 说明 |
|------|------|
| `server.host` / `port` | 客户端 WebSocket 监听地址 |
| `gateway.url` | Relay 中继地址（原为 Gateway 直连） |
| `gateway.node_id` | 当前节点标识 |
| `gateway.heartbeat_interval_secs` | 心跳间隔（秒） |
| `gateway.reconnect_base_ms` | 重连初始延迟（毫秒） |
| `gateway.reconnect_max_ms` | 重连最大延迟（毫秒） |

### 客户端 `config/client.toml`

```toml
[server]
url = "ws://127.0.0.1:9800"
```

### 中继 `config/relay.toml`

```toml
[relay]
server_listen = "0.0.0.0:19000"
agent_listen = "0.0.0.0:19001"
```

| 字段 | 说明 |
|------|------|
| `relay.server_listen` | myclaw-server 连接的监听地址 |
| `relay.agent_listen` | myclaw-agent 连接的监听地址 |

### 代理 `config/agent.toml`

```toml
[agent]
relay_url = "ws://YOUR_SERVER_IP:19001"
gateway_url = "ws://127.0.0.1:18789"
agent_id = "myclaw-agent-01"
reconnect_base_ms = 1000
reconnect_max_ms = 30000
```

| 字段 | 说明 |
|------|------|
| `agent.relay_url` | 云服务器 relay 的 agent 端口地址 |
| `agent.gateway_url` | 本地 OpenClaw Gateway 地址 |
| `agent.agent_id` | 代理标识 |
| `agent.reconnect_base_ms` | 重连初始延迟（毫秒） |
| `agent.reconnect_max_ms` | 重连最大延迟（毫秒） |

---

## 架构

### 项目结构

```
myclaw/
├── Cargo.toml                 # workspace 定义
├── config/
│   ├── server.toml
│   ├── client.toml
│   ├── relay.toml
│   └── agent.toml
├── myclaw-common/             # 共享库
│   └── src/
│       ├── lib.rs
│       ├── protocol.rs        # 消息协议定义
│       └── error.rs           # 错误类型
├── myclaw-server/             # 频道服务器
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── gateway.rs
│       ├── server.rs
│       └── router.rs
├── myclaw-relay/              # 中继层
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── bridge.rs          # 共享状态 + 通道转发
│       ├── server_side.rs     # 接受 server 连接
│       └── agent_side.rs      # 接受 agent 连接
├── myclaw-agent/              # 隧道代理
│   └── src/
│       ├── main.rs            # 入口 + 断线重连循环
│       ├── config.rs
│       └── tunnel.rs          # 双向桥接 relay ↔ gateway
└── myclaw-client/             # TUI 客户端
    └── src/
        ├── main.rs
        ├── config.rs
        ├── ws.rs
        └── tui.rs
```

### 消息流

```
用户输入
  │
  ▼
TUI ──ClientMessage::Chat──► ws.rs ──WebSocket──► server.rs
                                                     │
                                          handle_client_msg()
                                                     │
                                                     ▼
                                                 router.rs
                                            send_to_gateway()
                                                     │
                                          GatewayFrame::ChatRequest
                                                     │
                                                     ▼
                                               gateway.rs ──WS──► relay (server_side)
                                                                       │
                                                                  透明转发
                                                                       │
                                                                       ▼
                                                                  relay (agent_side) ──WS──► agent (tunnel)
                                                                                                │
                                                                                                ▼
                                                                                         OpenClaw Gateway
                                                                                                │
                                                                                    GatewayFrame::ChatResponse
                                                                                                │
                                                                                          原路返回
                                                                                                │
                                                                                                ▼
ws.rs ◄──WebSocket──── server.rs ◄── relay ◄── agent          (流式分块，done=true 结束)
  │
  ▼
TUI 渲染
```

### 协议概览

三层消息类型，均为 JSON + `type` 标签序列化：

| 层 | 类型 | 方向 |
|----|------|------|
| `ClientMessage` | `chat` / `ping` | Client → Server |
| `ServerMessage` | `chat_reply` / `error` / `pong` / `status` | Server → Client |
| `GatewayFrame` | `connect` / `connected` / `chat_request` / `chat_response` / `ping` / `pong` / `error` | Server ↔ Gateway |
| `RelayFrame` | `agent_hello` / `agent_welcome` | Agent ↔ Relay |

---

## TUI 操作

| 按键 | 功能 |
|------|------|
| `Enter` | 发送消息 |
| `Ctrl+C` | 退出 |
| `↑` / `↓` | 滚动消息 |

消息颜色：
- 🟦 **青色** `>` — 你发送的消息
- 🟩 **绿色** — AI 回复
- 🟨 **黄色** `*` — 系统通知

---

## 技术栈

| 组件 | 依赖 |
|------|------|
| 异步运行时 | tokio |
| WebSocket | tokio-tungstenite (native-tls) |
| 序列化 | serde + serde_json |
| 日志 | tracing + tracing-subscriber |
| CLI 参数 | clap |
| 配置 | toml |
| 终端 UI | ratatui + crossterm |
| ID 生成 | uuid v4 |
| 时间 | chrono |

---

## CI/CD

项目包含 GitHub Actions workflow，手动触发即可在 `ubuntu-latest` 上编译 Linux x86_64 release 并自动发布到 GitHub Releases。

```bash
# 或通过 GitHub API 触发
curl -X POST \
  "https://api.github.com/repos/oneadms/myclaw/actions/workflows/release.yml/dispatches" \
  -H "Authorization: token YOUR_TOKEN" \
  -d '{"ref":"main"}'
```

---

## License

MIT
