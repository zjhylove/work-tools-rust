# 双因素认证（auth）

> TOTP 动态验证码生成与管理，兼容 Google Authenticator

## 功能特性

- 生成 RFC 6238 TOTP 6 位验证码，兼容 Google Authenticator
- 支持多个认证条目管理（添加/编辑/删除）
- 每个条目可配置算法（SHA1/SHA256/SHA512）、位数、时间周期
- 随机密钥生成（20 字节，Base32 编码）
- 验证码倒计时自动刷新，倒计时归零时自动获取新验证码
- 一键复制验证码到剪贴板
- 手动刷新验证码

## 使用方法

### 基本操作

1. **添加认证条目** -- 点击"添加"按钮，填写发行方（如 Google）、账户名称（如 user@example.com）、密钥（Base32 编码，可点击"生成"自动生成），配置算法/位数/周期后保存
2. **生成密钥** -- 在表单中点击"生成"按钮，自动生成 20 字节随机密钥（Base32 编码）
3. **查看验证码** -- 列表页实时显示每个条目的 6 位验证码和剩余有效时间
4. **复制验证码** -- 点击剪贴板图标复制当前验证码
5. **刷新验证码** -- 点击刷新图标强制重新生成验证码
6. **编辑条目** -- 点击编辑按钮修改发行方、名称、密钥等配置
7. **删除条目** -- 点击删除按钮，在模态弹窗中确认删除

### 配置项

| 参数 | 默认值 | 说明 |
|------|--------|------|
| algorithm | SHA1 | TOTP 算法（SHA1/SHA256/SHA512） |
| digits | 6 | 验证码位数 |
| period | 30 | 时间周期（秒），通常为 30 秒 |

## 技术实现

### 后端（Rust）

**模块结构**：
- `src/lib.rs` -- 插件主入口，包含 TOTP 核心算法和 Plugin trait 实现

**TOTP 算法流程**：
1. Base32 解码密钥为原始字节
2. 计算时间步：`当前 Unix 时间 / period`
3. 时间步转为 8 字节大端序数组
4. 计算 HMAC-SHA1（密钥, 时间步），得到 20 字节哈希
5. 动态截取：取哈希最后 4 位作为偏移量，取 4 字节，去掉最高位
6. 取模 `10^digits` 得到验证码，左侧补零到指定位数

**核心数据结构**：

| 结构体 | 用途 |
|--------|------|
| `AuthEntry` | 认证条目（id, name, issuer, secret, algorithm, digits, period, created_at, updated_at） |
| `AuthData` | 顶层存储结构，包含 `entries: Vec<AuthEntry>` |

**handle_call 方法列表**：

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `list_entries` | `{}` | `[{id, name, issuer, secret, ...}]` | 列出所有认证条目 |
| `add_entry` | `{entry: {name, issuer, secret, algorithm, digits, period, ...}}` | 新建的条目对象 | 添加认证条目（自动生成 UUID） |
| `update_entry` | `{entry: {id, name, issuer, ...}}` | 更新后的条目对象 | 更新认证条目 |
| `delete_entry` | `{id: string}` | `{success: true}` | 删除认证条目 |
| `generate_secret` | `{}` | Base32 编码的密钥字符串 | 生成 20 字节随机密钥 |
| `generate_totp` | `{secret, digits, period}` | `{code: string}` | 生成 TOTP 验证码 |

**数据存储**：
- 使用 `PluginStorage` 持久化到 `~/.worktools/history/plugins/auth.json`
- 密钥以 Base32 明文存储（TOTP 标准编码格式）

**依赖的外部库**：

| crate | 用途 |
|-------|------|
| `hmac` | HMAC-SHA1 消息认证码 |
| `sha1` | SHA-1 哈希算法 |
| `base32` | Base32 编解码（TOTP 密钥标准编码） |
| `getrandom` | 安全随机数生成（密钥生成） |
| `uuid` | 条目唯一 ID |
| `chrono` | 时间戳 |
| `rand` | 随机数辅助 |
| `worktools-plugin-api` | Plugin trait 和 PluginStorage |

### 前端（React + TypeScript）

**组件结构**：
- `App.tsx` -- 主组件，包含列表视图、表单视图、删除确认弹窗
  - 列表视图：条目列表，每项显示名称、发行方、验证码、倒计时
  - 表单视图：发行方、账户名称、密钥（含"生成"按钮）、算法/位数/周期配置
  - 模态弹窗：`wt-modal-overlay` 删除确认

**验证码刷新机制**：
- 组件挂载时加载条目列表并批量生成初始验证码
- 每秒 tick 更新倒计时（`setTotpMap` 中递减 `remaining_seconds`）
- 倒计时归零时，将该条目加入 `needsRefresh` 队列，通过 `queueMicrotask` 异步调用后端重新生成
- 使用 `isMountedRef` 防止组件卸载后的 state 更新
- 使用 `entriesRef` 同步最新条目列表，避免闭包陷阱

**pluginAPI.call 调用列表**：

| pluginId | method | 说明 |
|----------|--------|------|
| `auth` | `list_entries` | 加载认证条目列表 |
| `auth` | `add_entry` | 添加认证条目 |
| `auth` | `update_entry` | 更新认证条目 |
| `auth` | `delete_entry` | 删除认证条目 |
| `auth` | `generate_secret` | 生成随机密钥 |
| `auth` | `generate_totp` | 生成验证码 |

**特殊处理**：
- 验证码和密钥不输出到控制台日志（安全考虑）
- 表单验证：发行方、账户名称必填且最少 1 字符，密钥必填且最少 10 字符
- 使用 `useMemo` 计算表单有效性，避免不必要的重渲染
- 新增条目后立即通过 `setTimeout` 延迟生成验证码

**前端依赖**：
- React 18 + TypeScript + Vite 5
- 无额外第三方依赖

## 开发与调试

```bash
# Rust 后端
cargo check -p auth-plugin               # 类型检查
cargo test -p auth-plugin                 # 运行测试

# 前端
cd plugins/auth-plugin/frontend
npm run dev                               # 启动 Vite 开发服务器
npm run build                             # TypeScript 检查 + 构建
```

## 已知限制

- 目前仅实现 SHA1 算法的 TOTP 生成，SHA256/SHA512 选项在前端可选但后端 `generate_totp_internal` 仅使用 `Hmac<Sha1>`。如需支持其他算法，需根据 `algorithm` 参数动态选择哈希类型
- 密钥以 Base32 明文存储在本地 JSON 文件中，未做加密保护。建议配合操作系统文件权限管理
- 不支持从 `otpauth://` URI 扫码导入
- 不支持条目排序或分组
