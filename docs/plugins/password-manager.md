# 密码管理器（password-manager）

> 本地 AES-256 加密存储密码，支持增删改查、导入导出

## 功能特性

- AES-256 加密存储所有密码，密钥由 SHA-256 固定种子派生
- 密码条目增删改查，每个条目包含服务名称、用户名、密码、URL
- 按服务名/用户名/URL 关键词搜索过滤
- 密码显示/隐藏切换，一键复制到剪贴板
- JSON 格式批量导入导出，支持新旧两种文件格式
- 表单字段实时校验（必填、最小长度、URL 格式）
- 删除操作需模态弹窗二次确认

## 使用方法

### 基本操作

1. **添加密码** -- 点击工具栏"新建"按钮，填写服务名称、用户名、密码（必填）和网站链接（可选），点击"保存"
2. **编辑密码** -- 点击条目右侧编辑按钮，修改表单内容后点击"更新密码"
3. **删除密码** -- 点击条目右侧删除按钮，在模态弹窗中确认删除
4. **查看密码** -- 点击眼睛图标切换密码明文/密文显示
5. **复制密码** -- 点击剪贴板图标，密码自动复制到系统剪贴板
6. **打开链接** -- 点击链接图标，通过 Tauri shell 或浏览器打开关联 URL
7. **搜索** -- 在搜索框输入关键词，按服务名/用户名/URL 模糊匹配

### 导入导出

- **导出** -- 点击"导出"按钮，选择目标目录，生成 `passwords-backup-YYYY-MM-DD.json` 文件
- **导入** -- 点击"导入"按钮，选择 JSON 文件。支持两种格式：
  - 旧格式：数组 `[{...}, {...}]`
  - 新格式：对象 `{"entries": [{...}]}`，按 ID 去重导入

## 技术实现

### 后端（Rust）

**模块结构**：
- `src/lib.rs` -- 插件主入口，实现 Plugin trait，定义 handle_call 方法分发
- `src/crypto.rs` -- AES-256 加密/解密模块，ECB 模式 + PKCS7 填充

**核心数据结构**：

| 结构体 | 用途 |
|--------|------|
| `PasswordEntry` | 密码条目（id, url, service, username, password, created_at, updated_at） |
| `PasswordData` | 顶层存储结构，包含 `entries: Vec<PasswordEntry>` |
| `PasswordEncryptor` | 加密器，内含 `Aes256` cipher 实例 |
| `CryptoConfig` | 加密配置（预留扩展） |

**handle_call 方法列表**：

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `list_passwords` | `{}` | `[{id, url, service, username, password, created_at, updated_at}]` | 列出所有密码（解密后） |
| `add_password` | `{service, username, password, url?}` | 新建的条目对象 | 添加密码（加密存储） |
| `update_password` | `{id, service, username, password, url?}` | 更新后的条目对象 | 更新密码 |
| `delete_password` | `{id}` | `{success: true}` | 删除指定密码 |
| `clear_all_passwords` | `{}` | `{success: true}` | 清空所有密码 |
| `export_passwords` | `{}` | `{data: string}` | 导出为 JSON 字符串 |
| `import_passwords` | `{data: string}` | `{success: true}` | 导入 JSON 数据（按 ID 去重） |

**加密方案**：
- 算法：AES-256 ECB 模式 + PKCS7 填充
- 密钥派生：SHA-256(`WorkToolsPasswordManager2024InternalKey` + `SALT_FIX_FOR_LOCAL_ENCRYPTION`)
- 输出编码：Hex 十六进制字符串
- 加密器为 `once_cell::sync::Lazy` 全局单例，线程安全

**数据存储**：
- 使用 `PluginStorage` 持久化到 `~/.worktools/history/plugins/password-manager.json`
- 保存时通过 `save_json_preserving` 保留 `salt` 和 `validation_token` 字段

**依赖的外部库**：

| crate | 用途 |
|-------|------|
| `aes` | AES-256 块加密 |
| `sha2` | SHA-256 哈希（密钥派生） |
| `hex` | 十六进制编解码 |
| `uuid` | 生成条目唯一 ID |
| `chrono` | 时间戳（RFC 3339 格式） |
| `once_cell` | Lazy 全局单例 |
| `serde` / `serde_json` | JSON 序列化 |
| `worktools-plugin-api` | Plugin trait 和 PluginStorage |

### 前端（React + TypeScript）

**组件结构**：
- `App.tsx` -- 主组件，包含列表视图、表单视图、删除确认弹窗
  - 列表视图：工具栏（新建/导入/导出/搜索） + 密码条目列表 + 底部统计
  - 表单视图：动态字段渲染（基于 `passwordFormFields` 配置）
  - 模态弹窗：`wt-modal-overlay` + `wt-modal` 样式

**pluginAPI.call 调用列表**：

| pluginId | method | 说明 |
|----------|--------|------|
| `password-manager` | `list_passwords` | 加载密码列表 |
| `password-manager` | `add_password` | 添加密码 |
| `password-manager` | `update_password` | 更新密码 |
| `password-manager` | `delete_password` | 删除密码 |
| `password-manager` | `export_passwords` | 导出密码数据 |

其他 API 调用：
- `window.pluginAPI.open_folder_dialog()` -- 选择导出目录
- `window.pluginAPI.write_file()` -- 写入导出文件
- `window.pluginAPI.open_url()` -- 打开关联链接

**特殊处理**：
- 剪贴板复制：优先使用 `navigator.clipboard.writeText()`，降级到 `document.execCommand("copy")`
- URL 打开：优先使用 `pluginAPI.open_url()`，降级到 `window.open()`
- 表单校验：实时逐字段校验 + 提交前全量校验，使用 `WorkTools.toast` 反馈
- 导入文件：动态创建 `<input type="file">` 元素，读取后自动清理 DOM

**前端依赖**：
- React 19 + TypeScript + Vite 5
- 无额外第三方依赖

## 开发与调试

```bash
# Rust 后端
cargo check -p password-manager        # 类型检查
cargo test -p password-manager          # 运行测试（含 crypto 加密解密测试）

# 前端
cd plugins/password-manager/frontend
npm run dev                             # 启动 Vite 开发服务器
npm run build                           # TypeScript 检查 + 构建
```

## 已知限制

- 加密使用 ECB 模式，相同的明文会产生相同的密文，安全性弱于 CBC/GCM 模式。生产环境建议改用系统密钥库（macOS Keychain / Windows Credential Manager）或用户主密码 + PBKDF2/Argon2 密钥派生
- 加密密钥由硬编码种子派生，不同安装实例使用相同密钥
- 加密失败时优雅降级为明文存储（`encrypt_or_plain`），可能在不经意间暴露密码
- 单个条目无独立锁定/解锁机制
