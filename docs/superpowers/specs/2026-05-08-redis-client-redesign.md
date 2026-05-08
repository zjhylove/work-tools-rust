# Redis Client 插件重构设计

**日期**: 2026-05-08
**参考**: Another Redis Desktop Manager (qishibo/AnotherRedisDesktopManager)
**目标**: 参考 ARDM 重构 redis-client 插件，遵循当前插件标准

---

## 1. 范围与决策

| 决策项 | 结论 |
|--------|------|
| 功能范围 | 适度扩展（连接增强 + 数据查看器 + 批量操作） |
| 主布局 | 双面板优化版（左侧连接选择器+Key树，右侧多Tab值查看器） |
| SSH 隧道 | 插件独立实现，使用 ssh2 crate |
| 数据查看器 | JSON格式化、HEX二进制、值内搜索、批量编辑 |
| 连接模式 | 单活跃连接，通过下拉框切换 |
| Cluster | 基本透明支持（自动拓扑发现+命令重定向） |

---

## 2. 后端架构

### 2.1 文件结构

```
plugins/redis-client/src/
├── lib.rs              # Plugin trait 实现 + handle_call 方法路由
├── connection.rs       # 连接管理抽象（Direct / SSH / Cluster）
├── ssh_tunnel.rs       # SSH 隧道（ssh2 crate，本地端口转发）
├── operations.rs       # Redis 操作（scan / CRUD / JSON检测 / HEX dump / 批量）
├── hex.rs              # XOR 混淆 + HEX 编解码（已有）
└── tests.rs            # 测试
```

### 2.2 连接配置模型

```rust
struct ConnectionConfig {
    id: String,
    name: String,
    color: Option<String>,          // 颜色标记 (8色)
    host: String,
    port: u16,
    password_obfuscated: String,    // XOR 混淆存储
    db: i64,
    ssh: Option<SshConfig>,         // SSH 隧道（可选）
    cluster: Option<ClusterConfig>, // Cluster 模式（可选）
}

struct SshConfig {
    host: String,
    port: u16,
    username: String,
    auth: SshAuth,                  // Password 或 KeyPath
    timeout_secs: u32,
}
```

### 2.3 handle_call 方法表

**连接管理** (新增/加强):
| 方法 | 说明 | 变更 |
|------|------|------|
| `connect` | 建立连接（直连/SSH/Cluster） | 加强：支持 SSH 和 Cluster |
| `disconnect` | 断开连接 | 保持 |
| `get_connection_info` | 当前连接信息 | 加强：增加 name/color |
| `save_connection` | 新建/更新连接配置（upsert by id） | 加强：完整配置字段 |
| `list_connections` | 列出全部已保存连接 | 加强：完整字段 |
| `delete_connection` | 删除连接 | 保持 |
| `test_connection` | **新增**：连通性验证（SSH可达+Redis PING） | 新增 |
| `get_saved_password` | 获取解混淆后的密码 | 保持 |

**Key 操作** (保持 + 新增):
| 方法 | 说明 |
|------|------|
| `scan_keys` | SCAN 扫描（保持） |
| `get_key_info` | 获取 key 类型/TTL/长度（保持） |
| `delete_key` | 删除 key（保持） |
| `delete_keys` | **新增**：批量删除 |
| `rename_key` | 重命名 key（保持） |
| `set_ttl` | 设置 TTL（保持） |

**数据类型操作** (保持 + 新增):
| 方法 | 说明 |
|------|------|
| `get_string` / `set_string` | String 读写（保持） |
| `get_hash` / `set_hash_field` / `del_hash_field` | Hash 操作（保持） |
| `get_list` / `lpush` / `rpush` / `lrem` | List 操作（保持） |
| `get_set` / `sadd` / `srem` | Set 操作（保持） |
| `get_zset` / `zadd` / `zrem` | ZSet 操作（保持） |
| `search_value` | **新增**：在 key 值内搜索 |
| `hex_dump` | **新增**：HEX 转储（指定 key 的前 N 字节） |

---

## 3. 前端架构

### 3.1 组件树

```
frontend/src/
├── main.tsx
├── App.tsx                     # 顶层状态路由
├── App.css                     # 全局布局样式（全部使用 var(--xxx) 令牌）
├── components/
│   ├── ConnectView.tsx         # 未连接时的连接面板
│   ├── WorkspaceView.tsx       # 连接后的双面板工作区
│   ├── ConnectionBar.tsx       # 左侧顶部：连接下拉 + 状态指示
│   ├── KeyPanel.tsx            # 左侧：搜索框 + Key 树
│   ├── KeyTree.tsx             # Key 树组件
│   ├── DetailPanel.tsx         # 右侧：值查看器主容器
│   ├── DetailToolbar.tsx       # 值查看器工具栏（搜索/格式化/HEX切换）
│   ├── viewers/
│   │   ├── StringViewer.tsx    # textarea + JSON 自动检测
│   │   ├── HashViewer.tsx      # 表格 + 搜索 + 批量
│   │   ├── ListViewer.tsx
│   │   ├── SetViewer.tsx
│   │   ├── ZSetViewer.tsx
│   │   └── HexViewer.tsx       # HEX 转储视图
│   ├── modals/
│   │   ├── ConnectionEdit.tsx  # 连接编辑对话框
│   │   └── DeleteConfirm.tsx   # 删除确认
│   └── ConnectionManager.tsx   # 连接管理全页视图
├── hooks/
│   ├── useConnection.ts        # 连接生命周期
│   ├── useKeys.ts              # Key 扫描 + 树构建
│   └── useKeyDetail.ts         # Key 详情 + 值加载
├── utils/
│   ├── tree.ts                 # buildTree 逻辑
│   └── json.ts                 # JSON 检测 + 格式化
└── types.ts                    # TypeScript 接口定义
```

### 3.2 App 状态路由

```tsx
function App() {
  const [view, setView] = useState<'connect' | 'workspace' | 'manager'>('connect');
  // view === 'connect'  → <ConnectView />
  // view === 'workspace'→ <WorkspaceView />
  // view === 'manager'  → <ConnectionManager />
}
```

### 3.3 组件说明

**ConnectView** — 快速连接表单 + 已保存连接列表，点击连接进入 workspace，点击"管理连接"进入 manager。

**ConnectionBar** — 下拉框切换已保存连接 + 绿色/红色状态指示点 + 断开按钮 + 管理连接入口。

**DetailPanel** — 根据 key 类型路由到对应 Viewer，提供工具栏（搜索框、JSON/HEX 切换、批量操作按钮）。支持多 Tab 切换已打开的 key。

**ConnectionManager** — 全页列表视图，每个连接卡片显示名称/颜色标记/Host/Port/SSH 信息，提供编辑、删除操作，右上角新建按钮。

**ConnectionEdit** — Modal 表单，包含所有连接字段，SSH/Cluster 通过 checkbox 展开。底部测试连接按钮。

### 3.4 数据流

```
WorkspaceView
├── useConnection
│   ├── connected: bool
│   ├── connectionInfo: {name, host, port, db, color}
│   ├── connect(id, password) → call('connect', ...)
│   └── disconnect() → call('disconnect', ...)
├── useKeys
│   ├── keys: KeyInfo[]
│   ├── tree: TreeNode[]           ← useMemo(buildTree, [keys])
│   ├── scan(pattern, append?)
│   └── deleteSelectedKeys(keys[])
├── useKeyDetail
│   ├── selectedKey / keyDetail / valueData
│   ├── openTabs: KeyInfo[]        ← 多 Tab 状态
│   ├── select(key) / deleteKey() / refresh()
│   └── searchInValue(query)
```

---

## 4. 样式原则

- 所有颜色使用 `var(--xxx)` 设计令牌，禁止硬编码
- 字体使用 `var(--font-sans)` / `var(--font-mono)`
- 间距/圆角/阴影统一使用 tokens.css 变量
- 双面板布局：左侧 key-panel 280px，右侧 detail-panel flex:1
- 连接下拉框样式参考现有 .panel-header 按钮风格

---

## 5. 测试策略

| 层 | 内容 | 命令 |
|----|------|------|
| Rust 单元测试 | `hex.rs` 中混淆/解混淆、`operations.rs` 中 JSON 检测/HEX 编码 | `cargo test -p redis-client` |
| Rust 集成测试 | 连接/scan/crud（需本地 Redis） | `cargo test -p redis-client -- --ignored` |
| 前端 | TypeScript 类型检查 | `cd plugins/redis-client/frontend && npx tsc --noEmit` |

---

## 6. Cargo.toml 依赖变更

新增依赖：
```toml
ssh2 = "0.9"      # SSH 隧道
```

---

## 7. 不纳入范围

以下 ARDM 功能明确不在此次重构中实现：
- Sentinel 模式
- RedisJSON / MsgPack / Protobuf 解码
- 内置命令行 (CLI)
- Subscribe / Monitor
- 内存分析
- 慢日志查询
- Stream 类型支持
