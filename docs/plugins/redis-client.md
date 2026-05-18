# Redis 客户端（redis-client）

> Redis 数据库管理工具，支持 Key 浏览、String/Hash/List/Set/ZSet 操作、SSH 隧道连接

## 功能特性

- 多连接管理：保存、编辑、删除 Redis 连接配置，支持颜色标记
- 三种连接模式：直连、SSH 隧道（密码/密钥认证）、Cluster 集群
- Key 浏览：SCAN 命令分页扫描，冒号分隔符树形结构展示
- 五种数据类型操作：String、Hash、List、Set、ZSet 的完整 CRUD
- 值搜索：在 Key 的值中搜索匹配文本
- Hex Dump：查看 Key 的原始字节（十六进制）
- 连接管理：测试连接、保存密码、快速连接
- TTL 管理：查看和设置 Key 的过期时间
- Key 管理：删除、批量删除、重命名

## 使用方法

### 基本操作

1. 打开插件后进入连接页面，可从已保存的连接中选择，或点击"快速连接"输入 host/port/db/password
2. 连接成功后进入工作区：
   - 左侧面板：Key 树形浏览，使用 SCAN 命令按 pattern 搜索
   - 右侧面板：选中 Key 后显示类型、TTL 和值详情
3. 点击底部"管理连接"按钮可添加/编辑/删除连接配置

### 配置项

每个保存的连接包含：
- **name**：连接名称
- **color**：颜色标记（8 种预设颜色）
- **host**：Redis 主机地址，默认 `127.0.0.1`
- **port**：Redis 端口，默认 `6379`
- **db**：数据库编号，默认 `0`
- **password**：连接密码（XOR 混淆存储）
- **ssh**（可选）：SSH 隧道配置
  - host / port / username
  - auth_type: `password` 或 `key`
  - password / key_path / key_passphrase
  - timeout_secs
- **cluster**（可选）：集群模式配置
  - seed_nodes: 种子节点列表（逗号分隔）

## 技术实现

### 后端（Rust）

- **模块结构**：
  - `lib.rs` - 主入口，`RedisClientPlugin` struct，所有 handle_call 方法
  - `connection.rs` - 连接配置模型（`ConnectionConfig`、`SshConfig`、`ClusterConfig`、`ConnectionMode`）
  - `operations.rs` - 高级操作（批量 Key 信息扫描、值搜索、Hex Dump）
  - `ssh_tunnel.rs` - SSH 隧道实现（基于 ssh2 库）
  - `hex.rs` - HEX 编解码 + XOR 混淆/解混淆（密码本地存储安全）

- **核心结构**：`RedisClientPlugin` 持有：
  - `client: Option<Client>` - 当前 Redis 连接
  - `current_config` - 当前连接配置
  - `storage: PluginStorage` - JSON 文件持久化
  - `saved_connections` - 已保存连接列表
  - `ssh_tunnel: Option<SshTunnel>` - 当前 SSH 隧道

- **handle_call 方法列表**：

| 分类 | 方法 | 参数 | 返回值 |
|---|---|---|---|
| 连接 | `connect` | `id?`, `host?`, `port?`, `db?`, `password?`, `ssh?`, `cluster?` | `{ ok, host, port, db }` |
| 连接 | `disconnect` | (无) | `{ ok }` |
| 连接 | `test_connection` | 同 connect | `{ ok }` |
| 连接 | `get_connection_info` | (无) | `{ connected, id, name, host, port, db }` |
| 连接 | `save_connection` | `id?`, `name`, `host?`, `port?`, `db?`, `password?`, `color?`, `ssh?`, `cluster?` | `{ ok, id }` |
| 连接 | `list_connections` | (无) | `{ connections: [...] }` |
| 连接 | `delete_connection` | `id` | `{ ok }` |
| 连接 | `get_saved_password` | `id` | `{ password, ssh_password, ssh_key_passphrase }` |
| Key | `scan_keys` | `cursor?`, `pattern?`, `count?` (默认 50, 最大 2000) | `{ cursor, keys: [{ key, type, ttl }] }` |
| Key | `get_key_info` | `key` | `{ key, type, ttl, length? }` |
| Key | `delete_key` | `key` | `{ deleted }` |
| Key | `delete_keys` | `keys` (array) | `{ deleted }` |
| Key | `rename_key` | `old`, `new` | `{ ok }` |
| Key | `set_ttl` | `key`, `seconds` | `{ ok }` |
| String | `get_string` | `key` | `{ value, is_json }` |
| String | `set_string` | `key`, `value` | `{ ok }` |
| Hash | `get_hash` | `key` | `{ fields: { field: value, ... } }` |
| Hash | `set_hash_field` | `key`, `field`, `value` | `{ ok }` |
| Hash | `del_hash_field` | `key`, `field` | `{ ok }` |
| List | `get_list` | `key`, `start?`, `stop?` | `{ items }` |
| List | `lpush` | `key`, `value` | `{ length }` |
| List | `rpush` | `key`, `value` | `{ length }` |
| List | `lrem` | `key`, `index` | `{ ok }` |
| Set | `get_set` | `key` | `{ members }` |
| Set | `sadd` | `key`, `member` | `{ added }` |
| Set | `srem` | `key`, `member` | `{ removed }` |
| ZSet | `get_zset` | `key` | `{ members: [{ member, score }] }` |
| ZSet | `zadd` | `key`, `score`, `member` | `{ added }` |
| ZSet | `zrem` | `key`, `member` | `{ removed }` |
| 操作 | `search_value` | `key`, `key_type`, `query` | `{ matches: [...] }` |
| 操作 | `hex_dump` | `key`, `max_bytes?` (默认 256, 最大 4096) | `{ hex, length }` |

- **SSH 隧道实现**：
  - 使用 `ssh2` crate 建立 SSH 连接
  - 本地随机端口绑定 `127.0.0.1:0`
  - 单线程非阻塞 I/O 转发循环，避免多线程竞争 Mutex
  - `SshTunnel` 在 Drop 时自动清理：设置 stop flag、断开 session、唤醒监听线程

- **密码安全**：本地密码使用 XOR 混淆（`worktools-redis-2026` 密钥），防止明文存储

- **数据存储**：使用 `PluginStorage` 持久化到 `~/.worktools/history/plugins/redis-client.json`

- **依赖库**：
  - `redis` 0.27 - Redis 客户端
  - `ssh2` 0.9 - SSH2 协议（隧道）
  - `uuid` 1 (v4) - 连接 ID 生成
  - `serde_json` / `serde` - JSON 序列化
  - `anyhow` - 错误处理
  - `tracing` 0.1 - 日志
  - `worktools-plugin-api` - 插件 trait + PluginStorage

### 前端（React + TypeScript）

- **组件结构**：
  - `App.tsx` - 根组件，视图切换（connect / workspace / manager）
  - `ConnectView.tsx` - 连接页面（已保存连接列表 + 快速连接）
  - `WorkspaceView.tsx` - 工作区（Key 面板 + 详情面板）
  - `ConnectionManager.tsx` - 连接配置管理
  - `KeyTree.tsx` - Key 树形浏览
  - `KeyPanel.tsx` - Key 列表面板
  - `DetailPanel.tsx` / `DetailToolbar.tsx` - Key 详情展示
  - `ContextMenu.tsx` - 右键菜单
  - `ConnectionBar.tsx` - 顶部连接信息栏
  - `Toast.tsx` - Toast 提示
  - `modals/` - 各种弹窗组件
  - `viewers/` - 各数据类型查看器

- **Hooks**：
  - `useConnection` - 管理连接状态（connect / disconnect）
  - `useKeys` - Key 扫描、树形构建、批量删除
  - `useKeyDetail` - 选中 Key 的详情加载和刷新

- **Utils**：
  - `tree.ts` - `buildTree()` 将扁平 Key 列表按冒号分隔符构建为树形结构
  - `api.ts` - `call()` 封装 pluginAPI.call，带 API 就绪等待和超时机制

- **pluginAPI.call 调用列表**：
  - 所有方法均通过 `api.ts` 的 `call()` 调用，pluginId 为 `redis-client`
  - 连接管理：`connect`, `disconnect`, `test_connection`, `save_connection`, `list_connections`, `delete_connection`, `get_saved_password`, `get_connection_info`
  - Key 操作：`scan_keys`, `get_key_info`, `delete_key`, `delete_keys`, `rename_key`, `set_ttl`
  - 数据类型：`get_string`, `set_string`, `get_hash`, `set_hash_field`, `del_hash_field`, `get_list`, `lpush`, `rpush`, `lrem`, `get_set`, `sadd`, `srem`, `get_zset`, `zadd`, `zrem`
  - 高级操作：`search_value`, `hex_dump`

- **特殊依赖**：无第三方 UI 库

- **macOS 适配**：视图切换时使用双 `requestAnimationFrame` 触发合成层重绘，解决 WKWebView srcdoc iframe 的布局刷新问题

## 开发与调试

```bash
# Rust 后端
cargo check -p redis-client
cargo test -p redis-client

# 前端
cd plugins/redis-client/frontend && npm run dev

# 完整构建
cd plugins/redis-client/frontend && npm run build
cargo build --release -p redis-client
```

## 已知限制

- Cluster 模式仅使用第一个 seed node 建立连接，未实现完整的集群路由
- 密码使用 XOR 混淆（非加密），仅防止明文暴露，不能抵御专业攻击
- SCAN 命令最多返回 2000 个 Key
- Hex Dump 最多读取 4096 字节
- SSH 隧道不支持 HTTP/SOCKS 代理跳转
- 无 Pub/Sub、Stream、HyperLogLog 等高级数据类型支持
