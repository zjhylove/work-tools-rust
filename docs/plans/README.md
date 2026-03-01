# Work Tools Platform - Rust 复刻计划

## 项目概述

将 Java 版本的 Work Tools Platform 完整复刻为 Rust 版本,使用 Tauri + Solid.js 技术栈构建跨平台桌面应用。

### 技术栈

- **后端**: Rust + Tauri 2.x
- **前端**: Solid.js + TypeScript
- **插件通信**: JSON-RPC 2.0 over stdin/stdout
- **数据存储**: JSON 文件(与 Java 版本兼容)
- **构建工具**: Cargo + Vite

### 架构设计

```
work-tools-rust/
├── tauri-app/              # Tauri 主应用
│   ├── src/               # Solid.js 前端
│   └── src-tauri/         # Rust 后端
│       ├── lib.rs         # Tauri 入口
│       ├── plugin_manager.rs  # 插件管理器
│       ├── commands.rs    # Tauri 命令
│       └── config.rs      # 配置管理
├── shared/                # 共享库
│   ├── types/            # 共享类型定义
│   └── rpc-protocol/     # JSON-RPC 协议
└── plugins/              # 插件实现
    ├── password-manager/
    ├── auth-plugin/
    ├── db-doc-plugin/
    ├── ip-forward-plugin/
    ├── object-storage-plugin/
    ├── ai-chat-plugin/
    ├── api-doc-plugin/
    └── db-router-plugin/
```

## 插件列表 (8 个)

### ✅ 已完成

1. **password-manager** - 密码管理器
   - 状态: ✅ 完成
   - 功能:
     - 密码增删改查
     - 搜索过滤
     - 表单验证
     - JSON 文件存储

2. **auth-plugin** - 双因素验证 (TOTP)
   - 状态: ✅ 基础完成
   - 功能:
     - TOTP 验证码生成
     - 密钥生成
     - QR 码生成
     - 待完成: 完整前端界面

### 🚧 待开发

3. **db-doc-plugin** - 数据库文档生成器
   - 功能:
     - 连接数据库
     - 读取表结构
     - 生成 Markdown 文档
     - 支持数据库: MySQL, PostgreSQL, SQLite

4. **ip-forward-plugin** - 端口转发工具
   - 功能:
     - 本地端口映射
     - 远程端口转发
     - SSH 隧道
     - 连接管理

5. **object-storage-plugin** - 对象存储管理
   - 功能:
     - S3 兼容接口
     - 文件上传/下载
     - Bucket 管理
     - 预签名 URL 生成

6. **ai-chat-plugin** - AI 对话助手
   - 功能:
     - 多 AI 模型支持
     - 对话历史管理
     - 流式响应
     - Prompt 模板

7. **api-doc-plugin** - API 文档生成器
   - 功能:
     - Spring Boot 注解解析
     - 接口文档生成
     - 在线调试
     - 导出 Markdown/HTML

8. **db-router-plugin** - 数据库路由
   - 功能:
     - 多数据源管理
     - 读写分离
     - 分库分表
     - SQL 路由

## 开发里程碑

### Phase 1: 基础架构 ✅
- [x] 项目结构搭建
- [x] Tauri 应用初始化
- [x] 插件管理器实现
- [x] JSON-RPC 协议定义
- [x] 共享类型库
- [x] 前端基础界面(左右分栏布局)

### Phase 2: 核心插件 ✅
- [x] password-manager 完整实现
- [x] auth-plugin 后端实现
- [x] auth-plugin 基础界面

### Phase 3: 完善功能 🚧
- [ ] auth-plugin 完整界面
  - [ ] 认证条目列表
  - [ ] 添加/编辑表单
  - [ ] TOTP 实时显示
  - [ ] 自动刷新倒计时
  - [ ] 点击复制验证码
- [ ] password-manager 增强
  - [ ] 密码强度指示器
  - [ ] 密码生成器
  - [ ] 导入/导出功能
  - [ ] 密码分类/标签

### Phase 4: 开发剩余插件 ⏳
- [ ] db-doc-plugin
- [ ] ip-forward-plugin
- [ ] object-storage-plugin
- [ ] ai-chat-plugin
- [ ] api-doc-plugin
- [ ] db-router-plugin

### Phase 5: 系统优化 ⏳
- [ ] 性能优化
- [ ] 错误处理完善
- [ ] 单元测试
- [ ] E2E 测试
- [ ] 文档完善
- [ ] 打包分发

## 技术要点

### 插件通信协议

插件通过 JSON-RPC 2.0 协议与主应用通信:

```json
// 请求
{"jsonrpc":"2.0","method":"get_info","params":{},"id":1}

// 响应
{"jsonrpc":"2.0","result":{"id":"password-manager","name":"密码管理器",...},"error":null,"id":1}
```

**重要**: 插件的日志必须输出到 stderr,保持 stdout 纯净用于 JSON-RPC 响应:

```rust
tracing_subscriber::fmt()
    .with_writer(std::io::stderr)  // 关键配置
    .init();
```

### 数据存储

所有数据存储在 `~/.worktools/` 目录:

```
~/.worktools/
├── plugins/           # 插件可执行文件
│   ├── password-manager/
│   └── auth-plugin/
├── data/             # 数据文件
│   ├── passwords.json
│   └── auth.json
└── config/           # 配置文件
    └── app.json
```

### UI 设计规范

- **配色方案**:
  - 侧边栏: `#1e1e1e` (深色)
  - 内容区: `#f5f5f5` (浅灰)
  - 主色调: `#0078d4` (蓝色)
  - 边框: `#e0e0e0`

- **布局**:
  - 左侧边栏: 250px 固定宽度
  - 右侧内容区: flex: 1 自适应
  - 圆角: 3px
  - 阴影: `0 1px 3px rgba(0,0,0,0.08)`

- **交互**:
  - 所有 `onClick` 事件需要添加 `preventDefault()` 和 `stopPropagation()`
  - 添加 `"user-select": "none"` 防止文本选择干扰
  - 悬停效果使用 `onMouseEnter`/`onMouseLeave`

## 已知问题和解决方案

### 1. 插件加载失败
**问题**: 只显示一个插件
**原因**: auth-plugin 日志输出到 stdout,污染 JSON-RPC 响应
**解决**: 将 tracing 日志重定向到 stderr

### 2. 点击无响应
**问题**: 点击插件菜单没有任何反应
**原因**: 事件冒泡和默认行为干扰
**解决**:
```typescript
onClick={(e) => {
  e.preventDefault();
  e.stopPropagation();
  // 处理逻辑
}}
```

### 3. 配色方案不美观
**问题**: 用户反馈"配色丑爆了"
**解决**: 采用 Windows 风格的专业配色,统一设计语言

## 开发规范

### Git 提交规范

```
<type>: <subject>

<body>

<footer>
```

类型:
- `feat`: 新功能
- `fix`: 修复 bug
- `refactor`: 重构
- `style`: 样式调整
- `docs`: 文档
- `test`: 测试
- `chore`: 构建/工具

### 代码审查清单

- [ ] 日志输出到 stderr (插件)
- [ ] 错误处理完善
- [ ] 类型安全 (TypeScript + Rust)
- [ ] UI 交互流畅
- [ ] 配色符合规范
- [ ] 代码注释清晰

## 测试策略

### 单元测试
- Rust 后端: `cargo test`
- 前端: vitest

### 集成测试
- 插件管理器测试
- JSON-RPC 通信测试

### E2E 测试
- Tauri 应用自动化测试
- 插件功能端到端测试

## 部署方案

### 开发构建
```bash
cd tauri-app
npm run tauri dev
```

### 生产构建
```bash
npm run tauri build
```

输出:
- macOS: `target/release/bundle/macos/tauri-app.app`
- Windows: `target/release/bundle/msi/`
- Linux: `target/release/bundle/deb/`

## 下一步工作

1. **auth-plugin 完整界面** (高优先级)
   - 实现认证条目列表
   - 表单界面
   - TOTP 实时显示和倒计时

2. **继续开发剩余插件**
   - 按优先级逐个完成
   - 每完成一个进行测试和提交

3. **优化和完善**
   - 性能优化
   - 用户体验改进
   - 文档完善

## 参考资源

- [Tauri 官方文档](https://tauri.app/)
- [Solid.js 文档](https://www.solidjs.com/)
- [JSON-RPC 2.0 规范](https://www.jsonrpc.org/specification)
- [Java 版本仓库](/Users/zj/Project/Java/work-tools-platform)

---

**文档版本**: 1.0
**最后更新**: 2026-03-01
**维护者**: zjhy
