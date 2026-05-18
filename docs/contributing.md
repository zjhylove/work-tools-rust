# 贡献指南 (Contributing Guide)

欢迎为 Work Tools Platform 贡献代码。本文档描述代码风格、提交规范、PR 流程和前端开发要求。

## 代码风格

### Rust

- 使用 `cargo fmt` 格式化代码，提交前必须运行
- 使用 `cargo clippy` 检查 lint 警告，提交前修复所有 clippy 警告
- 遵循项目中的既有风格：
  - 模块级文档注释用 `//!`
  - 公共函数和类型用 `///` 文档注释
  - 错误处理使用 `anyhow::Result` + `context()` 提供错误上下文
  - 使用 `tracing::info!` / `warn!` / `error!` 记录关键操作

### TypeScript / React

- 使用 `npx tsc --noEmit` 检查类型错误，提交前必须通过
- 遵循项目中的 ESLint 和 Prettier 配置
- 组件使用函数式组件 + Hooks

## Commit 规范

项目使用 [Conventional Commits](https://www.conventionalcommits.org/) 格式：

```
<type>(<scope>): <subject>

<body>
```

### Type 类型

| Type | 说明 | 示例 |
|------|------|------|
| `feat` | 新功能 | `feat(redis-client): add key scan support` |
| `fix` | Bug 修复 | `fix(plugin-manager): handle missing manifest gracefully` |
| `refactor` | 代码重构（不改变行为） | `refactor(logger): simplify ring buffer implementation` |
| `style` | 代码格式（不影响逻辑） | `style: fix formatting in commands.rs` |
| `docs` | 文档更新 | `docs: update plugin development guide` |
| `test` | 测试相关 | `test(password-manager): add encryption unit tests` |
| `chore` | 构建/工具变更 | `chore: update dependencies` |

### 规则

- subject 不超过 72 个字符
- 使用英文，小写开头，不加句号
- scope 通常为插件名或模块名（如 `password-manager`、`plugin-manager`）
- body 可选，用于说明改动原因

## PR 流程

1. **创建分支**: 从 `main` 创建功能分支，命名建议 `feat/<feature>` 或 `fix/<issue>`
2. **开发**: 遵循本文档的代码风格和前端开发规范
3. **测试**: 确保以下检查全部通过
   - `cargo test` -- 所有 workspace 测试通过
   - `cargo clippy` -- 无 clippy 警告
   - `cargo fmt --check` -- 格式化检查
   - `cd tauri-app && npx tsc --noEmit` -- TypeScript 类型检查
4. **提交**: 按规范编写 commit message
5. **创建 PR**: 标题遵循 Conventional Commits 格式，描述中包含改动说明和测试方法
6. **Code Review**: 至少一位 reviewer 审核通过
7. **合并**: 使用 squash merge 或常规 merge

## 测试要求

### Rust 测试

```bash
# 运行全部测试
cargo test

# 运行单个插件的测试
cargo test -p password-manager

# 按模块名过滤
cargo test -p password-manager -- crypto

# 按测试名过滤
cargo test -p db-router -- test_execute
```

新增功能必须包含对应的单元测试。Bug 修复建议附带回归测试。

### 前端测试

```bash
cd tauri-app && npx tsc --noEmit
```

确保 TypeScript 编译无错误。

## 前端开发规范

所有插件前端代码必须遵循以下规范。这些规范确保插件 UI 在浅色/暗色主题下表现一致，并与平台整体风格统一。

### CSS 变量

所有颜色**必须**使用 `var(--xxx)` 设计令牌，**禁止**硬编码色值。

```css
/* 正确 */
background: var(--bg-primary);
color: var(--text-primary);
border: 1px solid var(--border-color);

/* 禁止 */
background: #ffffff;
color: #333;
border: 1px solid rgba(0, 0, 0, 0.1);
```

常用设计令牌：
- `--bg-primary`, `--bg-secondary` -- 背景色
- `--text-primary`, `--text-secondary`, `--text-muted` -- 文字色
- `--border-color` -- 边框色
- `--color-primary`, `--color-danger`, `--color-success` -- 语义色

### 反馈提示 (Toast)

操作成功/失败使用统一的 toast 组件：

```javascript
WorkTools.toast.success('保存成功');
WorkTools.toast.error('操作失败');
WorkTools.toast.info('提示信息');
WorkTools.toast.warning('警告');
```

- Toast 自动消失 3 秒，点击可提前关闭
- 支持多条同时显示
- **禁止**自行实现 toast 或使用 `alert()`

### 表单校验

- 必须逐字段校验，失焦（blur）时触发校验
- 用户输入时清除本字段的校验错误
- 校验错误显示在本字段下方：

```javascript
WorkTools.FieldError.show(inputElement, '此项不能为空');
WorkTools.FieldError.hide(inputElement);
```

- 提交前全量校验，有任一错误不提交
- **禁止**用 toast 显示校验错误
- **禁止**使用原生 `alert()` 或 `confirm()`

### 组件规范

**按钮样式类**:
- `.wt-btn--primary` -- 主要操作按钮
- `.wt-btn--secondary` -- 次要操作按钮
- `.wt-btn--danger` -- 危险操作按钮（删除等）
- `.wt-btn--ghost` -- 幽灵按钮（无背景）

**模态框**:
- 删除/不可逆操作**必须**使用模态框确认
- 样式类：`.wt-modal-overlay` / `.wt-modal` / `.wt-modal-header` / `.wt-modal-body` / `.wt-modal-footer`

**表单**:
- 输入框：`.wt-form-input`
- 标签：`.wt-form-label`
- 容器：`.wt-form-group`

**其他**:
- 空状态：`.wt-empty-state`
- 加载态：按钮内嵌 `.wt-spinner` + `disabled` 属性
- 提交/导出等异步操作按钮**必须**有 loading 态

### 异步操作

所有异步操作按钮（提交、导出、加载等）在操作进行中必须：
1. 显示 loading spinner (`.wt-spinner`)
2. 设置 `disabled` 状态防止重复提交
3. 操作完成后恢复按钮状态

## 新增插件 Checklist

创建新插件时，确认以下事项：

- [ ] `Cargo.toml` 中 `crate-type = ["cdylib"]`
- [ ] 依赖 `worktools-plugin-api` 和 `tracing = "0.1"`
- [ ] 实现 `Plugin` trait 的所有必须方法
- [ ] 导出 `#[no_mangle] pub extern "C" fn plugin_create()`
- [ ] 创建 `manifest.json`（包含 files、assets 配置）
- [ ] 创建 `assets/` 目录，包含 `index.html`、`main.js`、`styles.css`
- [ ] CSS 使用 `var(--xxx)` 设计令牌，无硬编码颜色
- [ ] 使用 `PluginStorage` 管理持久化数据
- [ ] 关键操作记录 tracing 日志
- [ ] 包含单元测试
- [ ] `cargo clippy` 无警告
- [ ] `cargo fmt --check` 通过
- [ ] `npx tsc --noEmit` 通过（如果有前端代码）
