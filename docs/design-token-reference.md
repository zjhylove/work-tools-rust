# Design Token Reference

Work Tools 插件前端开发的 CSS 变量完整参考。所有变量定义于 `tauri-app/src/styles/tokens.css`，通过 iframe 注入机制传递给插件。

插件 CSS **必须**使用 `var(--xxx)` 设计令牌，禁止硬编码颜色值。硬编码色值的插件在暗色主题下会出现显示异常。

## 使用示例

```css
/* 正确 -- 使用设计令牌 */
.my-component {
  background: var(--bg-primary);
  color: var(--text-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  padding: var(--space-md) var(--space-lg);
  font-size: var(--font-size-base);
  transition: border-color var(--transition-base);
}

/* 错误 -- 禁止硬编码颜色值 */
.bad-component {
  background: #ffffff;      /* 浅色模式下看似正确，暗色模式下失效 */
  color: #333333;
  border: 1px solid #e5e7eb;
}
```

## 主题切换机制

- 浅色模式变量定义在 `:root` 选择器中。
- 暗色模式变量定义在 `[data-theme="dark"]` 选择器中。
- 插件 iframe 通过 `INJECTED_TOKENS` 接收包含两个选择器的完整令牌块。
- 切换主题时，主应用通过 `postMessage({ type: "theme", theme })` 通知所有已打开的 iframe。
- 令牌块在插件 styles.css 之后注入，确保令牌优先级最高。

---

## 主色调 (Accent)

| 变量名 | 浅色值 | 暗色值 | 用途 |
|--------|--------|--------|------|
| `--accent` | `#0066ff` | `#3b82f6` | 主按钮背景、链接色、选中态 |
| `--accent-hover` | `#0052cc` | `#60a5fa` | 主按钮悬停态 |
| `--accent-light` | `#eef3ff` | `#1e3a5f` | 选中行背景、高亮区域背景 |
| `--accent-ring` | `rgba(0, 102, 255, 0.15)` | `rgba(59, 130, 246, 0.25)` | 输入框聚焦时的外圈阴影 |

```css
.primary-button {
  background: var(--accent);
  color: var(--text-inverse);
  border-radius: var(--radius-md);
}
.primary-button:hover {
  background: var(--accent-hover);
}
input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-ring);
}
```

---

## 语义色 (Semantic Colors)

### Success

| 变量名 | 浅色值 | 暗色值 | 用途 |
|--------|--------|--------|------|
| `--success` | `#10b981` | `#34d399` | 成功图标、状态指示 |
| `--success-light` | `#ecfdf5` | `#064e3b` | 成功提示背景 |
| `--success-border` | `#a7f3d0` | `#065f46` | 成功提示边框 |
| `--success-text` | `#059669` | `#6ee7b7` | 成功提示文字 |

### Warning

| 变量名 | 浅色值 | 暗色值 | 用途 |
|--------|--------|--------|------|
| `--warning` | `#f59e0b` | `#fbbf24` | 警告图标、状态指示 |
| `--warning-light` | `#fffbeb` | `#78350f` | 警告提示背景 |
| `--warning-border` | `#fde68a` | `#92400e` | 警告提示边框 |
| `--warning-text` | `#b45309` | `#fcd34d` | 警告提示文字 |

### Error

| 变量名 | 浅色值 | 暗色值 | 用途 |
|--------|--------|--------|------|
| `--error` | `#ef4444` | `#f87171` | 错误图标、错误状态 |
| `--error-light` | `#fef2f2` | `#7f1d1d` | 错误提示背景 |
| `--error-border` | `#fecaca` | `#991b1b` | 错误提示边框 |
| `--error-text` | `#b91c1c` | `#fca5a5` | 错误提示文字 |

```css
.status-banner {
  padding: var(--space-md) var(--space-lg);
  border-radius: var(--radius-md);
}
.status-banner.success {
  background: var(--success-light);
  border: 1px solid var(--success-border);
  color: var(--success-text);
}
.status-banner.error {
  background: var(--error-light);
  border: 1px solid var(--error-border);
  color: var(--error-text);
}
```

---

## 背景 (Background)

| 变量名 | 浅色值 | 暗色值 | 用途 |
|--------|--------|--------|------|
| `--bg-primary` | `#ffffff` | `#1a1b1e` | 页面主背景、输入框背景 |
| `--bg-secondary` | `#f8f9fa` | `#25262b` | 次级背景、卡片背景、工具栏 |
| `--bg-tertiary` | `#f1f3f5` | `#2c2e33` | 第三层背景、分组区域 |
| `--hover-bg` | `rgba(0, 0, 0, 0.04)` | `rgba(255, 255, 255, 0.05)` | 悬停态背景 |

```css
.page {
  background: var(--bg-primary);
}
.card {
  background: var(--bg-secondary);
  border-radius: var(--radius-md);
}
.list-item:hover {
  background: var(--hover-bg);
}
```

---

## 文字 (Text)

| 变量名 | 浅色值 | 暗色值 | 用途 |
|--------|--------|--------|------|
| `--text-primary` | `#1b1c1d` | `#e5e7eb` | 主文字、标题 |
| `--text-secondary` | `#6b7280` | `#9ca3af` | 次级文字、描述、标签 |
| `--text-tertiary` | `#9ca3af` | `#6b7280` | 提示文字、placeholder |
| `--text-inverse` | `#ffffff` | `#1a1b1e` | 反色文字（用于深色背景上的文字） |

注意：`--text-tertiary` 的浅色和暗色值是互换的关系 -- 浅色模式下是 `#9ca3af`，暗色模式下是 `#6b7280`。

```css
h1, h2, h3 {
  color: var(--text-primary);
}
.description {
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
}
.primary-button {
  background: var(--accent);
  color: var(--text-inverse);
}
```

---

## 边框 (Border)

| 变量名 | 浅色值 | 暗色值 | 用途 |
|--------|--------|--------|------|
| `--border-color` | `#e5e7eb` | `#373a40` | 主要边框、输入框边框、分割线 |
| `--border-light` | `#f1f3f5` | `#2c2e33` | 淡边框、次要分割线 |

```css
input, select, textarea {
  border: 1px solid var(--border-color);
}
.divider {
  border-top: 1px solid var(--border-light);
}
```

---

## 阴影 (Shadow)

| 变量名 | 浅色值 | 暗色值 | 用途 |
|--------|--------|--------|------|
| `--shadow-xs` | `0 1px 2px rgba(0,0,0,0.03)` | `0 1px 2px rgba(0,0,0,0.2)` | 微阴影，内嵌元素 |
| `--shadow-sm` | `0 1px 3px rgba(0,0,0,0.05), 0 1px 2px rgba(0,0,0,0.04)` | `0 1px 3px rgba(0,0,0,0.3)` | 小阴影，卡片 |
| `--shadow-md` | `0 4px 12px rgba(0,0,0,0.06), 0 2px 4px rgba(0,0,0,0.04)` | `0 4px 12px rgba(0,0,0,0.4)` | 中阴影，弹出面板 |
| `--shadow-lg` | `0 12px 32px rgba(0,0,0,0.08), 0 4px 8px rgba(0,0,0,0.04)` | `0 12px 32px rgba(0,0,0,0.5)` | 大阴影，下拉菜单 |
| `--shadow-xl` | `0 20px 48px rgba(0,0,0,0.1)` | `0 20px 48px rgba(0,0,0,0.6)` | 超大阴影，模态框 |

```css
.card {
  box-shadow: var(--shadow-sm);
}
.modal {
  box-shadow: var(--shadow-xl);
}
.dropdown {
  box-shadow: var(--shadow-lg);
}
```

---

## 圆角 (Border Radius)

以下变量在浅色和暗色主题下值相同，不随主题变化。

| 变量名 | 值 | 用途 |
|--------|-----|------|
| `--radius-xs` | `4px` | 小元素、tag |
| `--radius-sm` | `6px` | 输入框、小按钮 |
| `--radius-md` | `8px` | 卡片、按钮、输入框 |
| `--radius-lg` | `12px` | 大卡片、面板 |
| `--radius-xl` | `16px` | 模态框 |
| `--radius-2xl` | `20px` | 大型容器 |

---

## 间距 (Spacing)

以下变量在浅色和暗色主题下值相同，不随主题变化。

| 变量名 | 值 | 用途 |
|--------|-----|------|
| `--space-xs` | `4px` | 紧凑间距、图标与文字之间 |
| `--space-sm` | `8px` | 小间距 |
| `--space-md` | `12px` | 中间距，常用内边距 |
| `--space-lg` | `16px` | 大间距，常用段落间距 |
| `--space-xl` | `24px` | 超大间距，区块间距 |
| `--space-2xl` | `32px` | 最大间距，页面内边距 |

```css
.container {
  padding: var(--space-xl);
}
.form-group {
  margin-bottom: var(--space-lg);
}
.inline-items {
  gap: var(--space-sm);
}
```

---

## 字体 (Typography)

以下变量在浅色和暗色主题下值相同，不随主题变化。

### 字体族

| 变量名 | 值 | 用途 |
|--------|-----|------|
| `--font-sans` | `-apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans SC", "PingFang SC", "Microsoft YaHei", sans-serif` | 正文、UI 文字 |
| `--font-mono` | `"SF Mono", "Cascadia Code", "Fira Code", "JetBrains Mono", Consolas, monospace` | 代码、等宽内容 |

### 字号

| 变量名 | 值 | 用途 |
|--------|-----|------|
| `--font-size-xs` | `11px` | 辅助标签、时间戳 |
| `--font-size-sm` | `12px` | 次要文字、描述 |
| `--font-size-base` | `13px` | 正文基准字号 |
| `--font-size-md` | `14px` | 强调正文 |
| `--font-size-lg` | `16px` | 小标题 |
| `--font-size-xl` | `18px` | 标题 |
| `--font-size-2xl` | `24px` | 大标题 |

```css
body {
  font-family: var(--font-sans);
  font-size: var(--font-size-base);
  color: var(--text-primary);
}
code, .mono {
  font-family: var(--font-mono);
  font-size: var(--font-size-sm);
}
h1 {
  font-size: var(--font-size-2xl);
}
```

---

## 动画 (Animation)

以下变量在浅色和暗色主题下值相同，不随主题变化。

| 变量名 | 值 | 用途 |
|--------|-----|------|
| `--transition-fast` | `0.12s ease` | 按钮悬停、颜色变化 |
| `--transition-base` | `0.2s ease` | 输入框聚焦、边框变化 |
| `--transition-slow` | `0.3s ease` | 面板展开、复杂过渡 |
| `--ease-out` | `cubic-bezier(0.16, 1, 0.3, 1)` | 弹出动画缓动 |
| `--ease-in-out` | `cubic-bezier(0.65, 0, 0.35, 1)` | 开关切换缓动 |

```css
.button {
  transition: background var(--transition-fast), border-color var(--transition-fast);
}
.panel {
  transition: max-height var(--transition-slow) var(--ease-out);
}
```

---

## 其他 (Misc)

以下变量在浅色和暗色主题下值相同，不随主题变化。

| 变量名 | 值 | 用途 |
|--------|-----|------|
| `--sidebar-width` | `260px` | 侧边栏宽度（插件通常不需要使用） |
| `--icon-size-sm` | `16px` | 小图标尺寸 |
| `--icon-size-md` | `20px` | 中图标尺寸 |
| `--icon-size-lg` | `24px` | 大图标尺寸 |

---

## 主题切换时变化的变量

以下变量在浅色和暗色主题间具有不同的值：

- **主色调**: `--accent`, `--accent-hover`, `--accent-light`, `--accent-ring`
- **语义色**: `--success`, `--success-light`, `--success-border`, `--success-text`, `--warning`, `--warning-light`, `--warning-border`, `--warning-text`, `--error`, `--error-light`, `--error-border`, `--error-text`
- **背景**: `--bg-primary`, `--bg-secondary`, `--bg-tertiary`, `--hover-bg`
- **文字**: `--text-primary`, `--text-secondary`, `--text-tertiary`, `--text-inverse`
- **边框**: `--border-color`, `--border-light`
- **阴影**: `--shadow-xs`, `--shadow-sm`, `--shadow-md`, `--shadow-lg`, `--shadow-xl`

**不随主题变化的变量**: 圆角、间距、字体族、字号、动画、侧边栏宽度、图标尺寸。

---

## 预制组件样式类

主应用提供了以下预制 CSS 类，插件可以直接使用，无需自行定义样式。

### 按钮

```css
.wt-btn--primary    /* 主按钮 -- 蓝色背景 */
.wt-btn--secondary  /* 次要按钮 -- 灰色背景 */
.wt-btn--danger     /* 危险按钮 -- 红色背景 */
.wt-btn--ghost      /* 幽灵按钮 -- 透明背景 */
```

### 模态框

```css
.wt-modal-overlay   /* 遮罩层 */
.wt-modal           /* 模态框容器 */
.wt-modal-header    /* 头部 */
.wt-modal-body      /* 内容区域 */
.wt-modal-footer    /* 底部（操作按钮） */
```

### 表单

```css
.wt-form-input      /* 输入框 */
.wt-form-label      /* 标签 */
.wt-form-group      /* 表单组容器 */
```

### 其他

```css
.wt-spinner         /* 加载旋转动画 */
.wt-empty-state     /* 空状态占位 */
```

### 反馈

```javascript
// Toast 通知（通过 JavaScript API 调用）
WorkTools.toast.success('操作成功');
WorkTools.toast.error('操作失败');
WorkTools.toast.info('提示信息');
WorkTools.toast.warning('警告');

// 字段错误提示
WorkTools.FieldError.show(inputElement, '此字段不能为空');
WorkTools.FieldError.hide(inputElement);
```

---

## 硬编码颜色检查清单

在提交插件代码前，检查以下位置是否使用了硬编码颜色：

1. **CSS 文件中**: 搜索 `#` 开头的十六进制色值（如 `#fff`、`#333`、`#e5e7eb`）。
2. **CSS 文件中**: 搜索 `rgba(` 和 `rgb(` 函数调用（除了 `opacity` 属性）。
3. **CSS 文件中**: 搜索 `color:` 属性中直接使用颜色名（如 `red`、`blue`、`white`）。
4. **JS/TS 文件中**: 搜索 `style` 属性中的内联颜色值。
5. **JS/TS 文件中**: 搜索 `backgroundColor`、`color`、`borderColor` 等内联样式。

所有颜色值应替换为对应的 `var(--xxx)` 设计令牌。
