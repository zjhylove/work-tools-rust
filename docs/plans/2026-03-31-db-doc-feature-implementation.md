# db-doc 功能改进实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 完善 db-doc 插件的导出功能、前端交互体验和错误处理，使其成为一个完整可用的数据库文档生成工具。

**Architecture:** 后端新增 DocumentExporter trait 统一三种格式导出，前端增加步骤导航、导出配置面板、Toast 通知等交互组件。文件对话框通过 PluginPlaceholder 注入到 iframe 的 pluginAPI 中。

**Tech Stack:** Rust (quick-xml/zip for DOCX, printpdf for PDF, sqlx for DB), React 19 + TypeScript (纯 HTML/CSS，无 UI 框架), Tauri 2 (tauri-plugin-dialog for file dialog)

---

## Task 1: 删除 Enterprise 模板变体

**Files:**
- Modify: `plugins/db-doc/src/models/connection.rs:144-150`
- Modify: `plugins/db-doc/src/exporter/markdown.rs:48`

**Step 1: 修改 TemplateStyle 枚举**

在 `plugins/db-doc/src/models/connection.rs` 中，删除 `Enterprise` 变体：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TemplateStyle {
    Simple,
    Detailed,
}
```

**Step 2: 修改 MarkdownExporter 的 match**

在 `plugins/db-doc/src/exporter/markdown.rs:48`，将 `TemplateStyle::Detailed | TemplateStyle::Enterprise` 改为：

```rust
fn render_table(&self, table: &TableInfo) -> String {
    match self.template_style {
        TemplateStyle::Simple => self.render_simple(table),
        TemplateStyle::Detailed => self.render_detailed(table),
    }
}
```

**Step 3: 运行测试**

Run: `cargo test -p db-doc`
Expected: PASS（两个 markdown 测试仍然通过）

**Step 4: Commit**

```bash
git add plugins/db-doc/src/models/connection.rs plugins/db-doc/src/exporter/markdown.rs
git commit -m "refactor(db-doc): remove Enterprise template variant, keep Simple and Detailed only"
```

---

## Task 2: 添加 DocumentExporter trait 并重构导出逻辑

**Files:**
- Modify: `plugins/db-doc/src/exporter/mod.rs`
- Modify: `plugins/db-doc/src/exporter/markdown.rs`
- Modify: `plugins/db-doc/src/lib.rs:159-227`

**Step 1: 在 exporter/mod.rs 中定义 DocumentExporter trait**

```rust
mod markdown;

pub use markdown::MarkdownExporter;

use anyhow::Result;
use crate::models::{TableInfo, ExportConfig};

/// 文档导出器 trait
pub trait DocumentExporter {
    /// 导出文档，返回生成的文件路径
    fn export(&self, tables: &[TableInfo], config: &ExportConfig) -> Result<Vec<String>>;
}
```

**Step 2: 为 MarkdownExporter 实现 DocumentExporter trait**

在 `plugins/db-doc/src/exporter/markdown.rs` 末尾添加：

```rust
use super::DocumentExporter;

impl DocumentExporter for MarkdownExporter {
    fn export(&self, tables: &[TableInfo], config: &ExportConfig) -> Result<Vec<String>> {
        let output_path = std::path::PathBuf::from(&config.output_dir);
        std::fs::create_dir_all(&output_path)?;

        let file_path = output_path.join(format!(
            "数据库文档_{}.md",
            chrono::Local::now().format("%Y%m%d")
        ));
        self.export_tables(tables, &file_path)?;
        Ok(vec![file_path.to_string_lossy().to_string()])
    }
}
```

**Step 3: 重构 handle_export_docs 使用 trait**

在 `plugins/db-doc/src/lib.rs` 中替换 `handle_export_docs` 方法的导出部分：

```rust
fn handle_export_docs(&self, params: Value) -> Result<Value> {
    let config: ExportConfig = serde_json::from_value(params)?;

    // 获取连接配置
    let connections = self.storage.list_connections()?;
    let conn_config = connections
        .into_iter()
        .find(|c| c.id == config.connection_id)
        .ok_or_else(|| anyhow::anyhow!("连接配置不存在"))?;

    // 获取表信息
    let tables_info = match conn_config.db_type {
        DatabaseType::MySQL => {
            let extractor = database::MySqlExtractor;
            self.runtime
                .block_on(extractor.get_tables_info(&conn_config, &config.tables))?
        }
        DatabaseType::PostgreSQL => {
            let extractor = database::PostgresExtractor;
            self.runtime
                .block_on(extractor.get_tables_info(&conn_config, &config.tables))?
        }
    };

    // 选择导出器并导出
    let exported_files: Vec<String> = match config.format {
        ExportFormat::Markdown => {
            let exporter = exporter::MarkdownExporter::new(config.template);
            exporter.export(&tables_info, &config)?
        }
        ExportFormat::Word => {
            return Err(anyhow::anyhow!("Word 导出即将实现"));
        }
        ExportFormat::Pdf => {
            return Err(anyhow::anyhow!("PDF 导出即将实现"));
        }
    };

    // 保存导出历史
    let history = ExportHistory {
        id: uuid::Uuid::new_v4().to_string(),
        connection_name: conn_config.name,
        tables: config.tables.clone(),
        format: config.format,
        template: config.template,
        output_path: exported_files
            .first()
            .cloned()
            .unwrap_or_default(),
        exported_at: chrono::Utc::now().to_rfc3339(),
    };
    self.storage.add_export_history(history)?;

    Ok(serde_json::json!({
        "success": true,
        "files": exported_files,
        "count": exported_files.len()
    }))
}
```

**Step 4: 运行测试**

Run: `cargo test -p db-doc`
Expected: PASS

**Step 5: Commit**

```bash
git add plugins/db-doc/src/exporter/ plugins/db-doc/src/lib.rs
git commit -m "refactor(db-doc): add DocumentExporter trait and unify export flow"
```

---

## Task 3: 暴露文件对话框 API 到插件前端

**Files:**
- Modify: `tauri-app/src-tauri/src/commands.rs` (添加新命令)
- Modify: `tauri-app/src-tauri/src/main.rs` (注册新命令)
- Modify: `tauri-app/src/components/PluginPlaceholder.tsx:202-232` (注入新 API)

**Step 1: 在 commands.rs 中添加文件对话框命令**

```rust
/// 打开文件夹选择对话框
#[tauri::command]
pub async fn open_folder_dialog(
    title: Option<String>,
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let folder_path = app
        .dialog()
        .file()
        .add_filter("文件夹", &["*"])
        .blocking_pick_folder();

    Ok(folder_path.map(|p| p.to_string()))
}
```

**Step 2: 在 main.rs 中注册命令**

在 `tauri::Builder::default()` 的 `.invoke_handler(tauri::generate_handler![...])` 中添加 `open_folder_dialog`。

**Step 3: 在 PluginPlaceholder.tsx 中注入 open_folder_dialog**

在 pluginAPI 对象中添加：

```typescript
open_folder_dialog: async (title?: string) => {
    return await invoke("open_folder_dialog", { title: title || "选择导出目录" });
},
```

**Step 4: 验证编译**

Run: `cd tauri-app && cargo check`
Expected: 编译通过

**Step 5: Commit**

```bash
git add tauri-app/src-tauri/src/commands.rs tauri-app/src-tauri/src/main.rs tauri-app/src/components/PluginPlaceholder.tsx
git commit -m "feat: expose folder dialog API to plugin frontend"
```

---

## Task 4: CSS 变量系统和全局样式重构

**Files:**
- Modify: `plugins/db-doc/frontend/src/App.css`

**Step 1: 在 App.css 顶部添加 CSS 变量**

```css
:root {
  --color-primary: #1890ff;
  --color-primary-hover: #40a9ff;
  --color-primary-light: #e6f7ff;
  --color-primary-border: #91d5ff;
  --color-success: #52c41a;
  --color-success-light: #f6ffed;
  --color-error: #ff4d4f;
  --color-error-light: #fff2f0;
  --color-error-border: #ffccc7;
  --color-warning: #faad14;
  --color-warning-light: #fffbe6;
  --color-bg: #f5f5f5;
  --color-card: #ffffff;
  --color-text: #333333;
  --color-text-secondary: #666666;
  --color-text-tertiary: #999999;
  --color-border: #e8e8e8;
  --color-border-light: #f0f0f0;
  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.04);
  --transition: all 0.2s ease;
}
```

**Step 2: 替换所有硬编码颜色为变量**

在 App.css 中将所有 `#1890ff` → `var(--color-primary)`、`#40a9ff` → `var(--color-primary-hover)` 等进行替换。这是纯文本替换，不改变任何布局。

**Step 3: 验证前端构建**

Run: `cd plugins/db-doc/frontend && npm run build`
Expected: 构建成功

**Step 4: Commit**

```bash
git add plugins/db-doc/frontend/src/App.css
git commit -m "style(db-doc): introduce CSS variables system for consistent theming"
```

---

## Task 5: Toast 通知组件

**Files:**
- Modify: `plugins/db-doc/frontend/src/App.tsx`
- Modify: `plugins/db-doc/frontend/src/App.css`

**Step 1: 在 App.tsx 中添加 Toast 组件和状态**

添加 `toast` 状态和 `showToast` 辅助函数：

```typescript
interface ToastMessage {
  id: number
  type: 'success' | 'error' | 'info'
  message: string
}

// 在 App 组件内部
const [toasts, setToasts] = useState<ToastMessage[]>([])

const showToast = (type: ToastMessage['type'], message: string) => {
  const id = Date.now()
  setToasts(prev => [...prev, { id, type, message }])
  setTimeout(() => {
    setToasts(prev => prev.filter(t => t.id !== id))
  }, 3000)
}
```

**Step 2: 添加 Toast 渲染和 CSS**

在 `return` 中添加 Toast 容器：

```tsx
{toasts.length > 0 && (
  <div className="toast-container">
    {toasts.map(toast => (
      <div key={toast.id} className={`toast toast-${toast.type}`}>
        {toast.type === 'success' && '✓ '}
        {toast.type === 'error' && '✗ '}
        {toast.message}
      </div>
    ))}
  </div>
)}
```

Toast CSS：

```css
.toast-container {
  position: fixed;
  top: 16px;
  right: 16px;
  z-index: 1000;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.toast {
  padding: 12px 20px;
  border-radius: var(--radius-md);
  box-shadow: var(--shadow);
  font-size: 14px;
  animation: slideIn 0.3s ease;
  max-width: 400px;
}

.toast-success {
  background: var(--color-success-light);
  color: var(--color-success);
  border: 1px solid var(--color-success);
}

.toast-error {
  background: var(--color-error-light);
  color: var(--color-error);
  border: 1px solid var(--color-error-border);
}

.toast-info {
  background: var(--color-primary-light);
  color: var(--color-primary);
  border: 1px solid var(--color-primary-border);
}

@keyframes slideIn {
  from { transform: translateX(100%); opacity: 0; }
  to { transform: translateX(0); opacity: 1; }
}
```

**Step 3: 替换所有 alert() 调用为 showToast**

- `testConnection` 中的 `alert('连接成功!')` → `showToast('success', '连接成功!')`
- `testConnection` 中的 `alert('连接失败: '...)` → `showToast('error', '连接失败: '...)`
- `handleExport` 中的 `alert('导出成功!')` → `showToast('success', '导出成功!')`

**Step 4: 验证构建**

Run: `cd plugins/db-doc/frontend && npm run build`
Expected: 构建成功

**Step 5: Commit**

```bash
git add plugins/db-doc/frontend/src/App.tsx plugins/db-doc/frontend/src/App.css
git commit -m "feat(db-doc): add Toast notification component, replace alert() calls"
```

---

## Task 6: 步骤导航组件

**Files:**
- Modify: `plugins/db-doc/frontend/src/App.tsx`
- Modify: `plugins/db-doc/frontend/src/App.css`

**Step 1: 替换 header 中的 nav 为步骤条**

将现有的 `<nav className="nav">` 替换为 StepsIndicator：

```tsx
<div className="steps">
  <div
    className={`step ${viewMode === 'connections' ? 'active' : 'completed'}`}
    onClick={() => setViewMode('connections')}
  >
    <span className="step-number">1</span>
    <span className="step-label">连接管理</span>
  </div>
  <div className="step-line"></div>
  <div
    className={`step ${
      viewMode === 'tables'
        ? 'active'
        : selectedConnection
          ? 'completed'
          : ''
    }`}
    onClick={() => selectedConnection && setViewMode('tables')}
  >
    <span className="step-number">2</span>
    <span className="step-label">选择表</span>
  </div>
  <div className="step-line"></div>
  <div
    className={`step ${viewMode === 'preview' ? 'active' : ''}`}
  >
    <span className="step-number">3</span>
    <span className="step-label">预览 & 导出</span>
  </div>
</div>
```

**Step 2: 添加步骤条 CSS**

```css
.steps {
  display: flex;
  align-items: center;
  gap: 0;
}

.step {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: var(--transition);
  font-size: 14px;
  color: var(--color-text-tertiary);
}

.step.active {
  color: var(--color-primary);
  font-weight: 500;
}

.step.completed {
  color: var(--color-success);
}

.step-number {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  border: 2px solid currentColor;
}

.step.active .step-number {
  background: var(--color-primary);
  color: white;
  border-color: var(--color-primary);
}

.step.completed .step-number {
  background: var(--color-success);
  color: white;
  border-color: var(--color-success);
}

.step-line {
  width: 40px;
  height: 2px;
  background: var(--color-border);
}
```

**Step 3: 验证构建**

Run: `cd plugins/db-doc/frontend && npm run build`
Expected: 构建成功

**Step 4: Commit**

```bash
git add plugins/db-doc/frontend/src/App.tsx plugins/db-doc/frontend/src/App.css
git commit -m "feat(db-doc): add step navigation with progress indicator"
```

---

## Task 7: 连接管理优化

**Files:**
- Modify: `plugins/db-doc/frontend/src/App.tsx` (ConnectionForm + 连接列表)
- Modify: `plugins/db-doc/frontend/src/App.css`

**Step 1: 在连接列表中添加搜索框**

在 `connections-list` 的 `<h2>` 后添加搜索框：

```tsx
<input
  className="search-input"
  placeholder="搜索连接..."
  value={connectionSearch}
  onChange={(e) => setConnectionSearch(e.target.value)}
/>
```

添加状态：`const [connectionSearch, setConnectionSearch] = useState('')`

过滤逻辑：在 `connections.map` 前添加 `const filteredConnections = connections.filter(c => c.name.toLowerCase().includes(connectionSearch.toLowerCase()))`

**Step 2: 测试连接按钮添加 loading 状态**

在 App 组件中添加 `const [testingConnection, setTestingConnection] = useState(false)`

修改 `testConnection` 函数设置 testingConnection 状态。在连接列表的测试按钮中：

```tsx
<button
  onClick={() => testConnection(conn)}
  disabled={testingConnection}
>
  {testingConnection ? '测试中...' : '测试连接'}
</button>
```

并在连接列表项中添加测试结果状态：

```typescript
const [testResult, setTestResult] = useState<{id: string, success: boolean, message?: string} | null>(null)
```

测试成功后显示绿色 ✓，失败显示红色 ✗ + 错误信息。

**Step 3: 连接列表中添加删除按钮**

```tsx
<button
  className="btn-danger"
  onClick={async () => {
    await window.pluginAPI.call('db-doc', 'delete_connection', { id: conn.id })
    loadConnections()
    showToast('success', `已删除连接: ${conn.name}`)
  }}
>
  删除
</button>
```

**Step 4: 验证构建**

Run: `cd plugins/db-doc/frontend && npm run build`
Expected: 构建成功

**Step 5: Commit**

```bash
git add plugins/db-doc/frontend/src/App.tsx plugins/db-doc/frontend/src/App.css
git commit -m "feat(db-doc): add connection search, test loading state and delete button"
```

---

## Task 8: 表选择增强

**Files:**
- Modify: `plugins/db-doc/frontend/src/App.tsx` (tables 视图)
- Modify: `plugins/db-doc/frontend/src/App.css`

**Step 1: 添加表搜索和前缀选择**

```typescript
const [tableSearch, setTableSearch] = useState('')
const [prefixFilter, setPrefixFilter] = useState('')

const filteredTables = tables.filter(t =>
  t.toLowerCase().includes(tableSearch.toLowerCase())
)

const selectByPrefix = () => {
  if (!prefixFilter) return
  const newSelected = new Set(selectedTables)
  tables.forEach(t => {
    if (t.toLowerCase().startsWith(prefixFilter.toLowerCase())) {
      newSelected.add(t)
    }
  })
  setSelectedTables(newSelected)
}

const invertSelection = () => {
  const newSelected = new Set<string>()
  tables.forEach(t => {
    if (!selectedTables.has(t)) newSelected.add(t)
  })
  setSelectedTables(newSelected)
}
```

**Step 2: 在 tables-header 中添加搜索和批量操作**

```tsx
<div className="tables-toolbar">
  <input
    className="search-input"
    placeholder="搜索表名..."
    value={tableSearch}
    onChange={(e) => setTableSearch(e.target.value)}
  />
  <div className="batch-actions">
    <input
      className="prefix-input"
      placeholder="前缀筛选"
      value={prefixFilter}
      onChange={(e) => setPrefixFilter(e.target.value)}
    />
    <button onClick={selectByPrefix} disabled={!prefixFilter}>按前缀选择</button>
    <button onClick={invertSelection}>反选</button>
  </div>
</div>
```

**Step 3: 添加相关 CSS**

```css
.tables-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  gap: 12px;
}

.batch-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.prefix-input {
  padding: 6px 10px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-size: 13px;
  width: 120px;
}

.search-input {
  padding: 8px 12px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-size: 14px;
  width: 200px;
}
```

**Step 4: 将 `tables.map` 改为 `filteredTables.map`**

**Step 5: 验证构建**

Run: `cd plugins/db-doc/frontend && npm run build`
Expected: 构建成功

**Step 6: Commit**

```bash
git add plugins/db-doc/frontend/src/App.tsx plugins/db-doc/frontend/src/App.css
git commit -m "feat(db-doc): add table search, prefix filter and invert selection"
```

---

## Task 9: 导出配置面板

**Files:**
- Modify: `plugins/db-doc/frontend/src/App.tsx` (预览视图)
- Modify: `plugins/db-doc/frontend/src/App.css`

**Step 1: 添加导出配置状态和面板**

```typescript
const [showExportPanel, setShowExportPanel] = useState(false)
const [exportFormat, setExportFormat] = useState<'markdown' | 'word' | 'pdf'>('markdown')
const [exportTemplate, setExportTemplate] = useState<'simple' | 'detailed'>('detailed')
const [exporting, setExporting] = useState(false)
```

**Step 2: 导出面板 JSX**

在预览视图中，替换直接导出按钮为弹出面板：

```tsx
{showExportPanel && (
  <div className="modal-overlay" onClick={() => setShowExportPanel(false)}>
    <div className="export-panel" onClick={e => e.stopPropagation()}>
      <h3>导出配置</h3>

      <div className="form-group">
        <label>导出格式</label>
        <div className="radio-group">
          <label className={`radio-item ${exportFormat === 'markdown' ? 'selected' : ''}`}>
            <input type="radio" name="format" value="markdown"
              checked={exportFormat === 'markdown'}
              onChange={() => setExportFormat('markdown')} />
            Markdown
          </label>
          <label className={`radio-item ${exportFormat === 'word' ? 'selected' : ''}`}>
            <input type="radio" name="format" value="word"
              checked={exportFormat === 'word'}
              onChange={() => setExportFormat('word')} />
            Word
          </label>
          <label className={`radio-item ${exportFormat === 'pdf' ? 'selected' : ''}`}>
            <input type="radio" name="format" value="pdf"
              checked={exportFormat === 'pdf'}
              onChange={() => setExportFormat('pdf')} />
            PDF
          </label>
        </div>
      </div>

      <div className="form-group">
        <label>模板风格</label>
        <div className="radio-group">
          <label className={`radio-item ${exportTemplate === 'simple' ? 'selected' : ''}`}>
            <input type="radio" name="template" value="simple"
              checked={exportTemplate === 'simple'}
              onChange={() => setExportTemplate('simple')} />
            简洁 (字段、类型、说明)
          </label>
          <label className={`radio-item ${exportTemplate === 'detailed' ? 'selected' : ''}`}>
            <input type="radio" name="template" value="detailed"
              checked={exportTemplate === 'detailed'}
              onChange={() => setExportTemplate('detailed')} />
            详细 (含索引、默认值等)
          </label>
        </div>
      </div>

      <button
        className="primary"
        disabled={exporting}
        onClick={handleExportWithDialog}
      >
        {exporting ? '导出中...' : '选择目录并导出'}
      </button>
    </div>
  </div>
)}
```

**Step 3: 实现 handleExportWithDialog**

```typescript
const handleExportWithDialog = async () => {
  if (!selectedConnection || selectedTables.size === 0) return

  try {
    setExporting(true)

    // 调用文件对话框选择目录
    const folder = await window.pluginAPI.open_folder_dialog('选择导出目录')

    if (!folder) {
      setExporting(false)
      return
    }

    const result = await window.pluginAPI.call('db-doc', 'export_docs', {
      connection_id: selectedConnection.id,
      tables: Array.from(selectedTables),
      output_dir: folder,
      format: exportFormat,
      template: exportTemplate,
    }) as { success: boolean; files?: string[]; message?: string }

    if (result.success) {
      showToast('success', `导出成功! 共 ${result.files?.length || 0} 个文件`)
      setShowExportPanel(false)
    } else {
      showToast('error', '导出失败: ' + (result.message || '未知错误'))
    }
  } catch (e) {
    showToast('error', '导出失败: ' + (e instanceof Error ? e.message : '未知错误'))
  } finally {
    setExporting(false)
  }
}
```

**Step 4: 导出面板 CSS**

```css
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.4);
  z-index: 500;
  display: flex;
  align-items: center;
  justify-content: center;
}

.export-panel {
  background: var(--color-card);
  border-radius: var(--radius-lg);
  padding: 24px;
  width: 420px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.16);
}

.export-panel h3 {
  margin: 0 0 20px 0;
  font-size: 18px;
}

.radio-group {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.radio-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 14px;
  transition: var(--transition);
}

.radio-item.selected {
  border-color: var(--color-primary);
  background: var(--color-primary-light);
  color: var(--color-primary);
}

.radio-item input[type="radio"] {
  display: none;
}
```

**Step 5: 修改预览视图中的导出按钮**

将原来的 `<button onClick={handleExport}>` 改为 `<button onClick={() => setShowExportPanel(true)}>导出文档</button>`

**Step 6: 更新 window.pluginAPI 类型声明**

```typescript
interface Window {
  pluginAPI: {
    call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>
    open_folder_dialog: (title?: string) => Promise<string | null>
  }
}
```

**Step 7: 验证构建**

Run: `cd plugins/db-doc/frontend && npm run build`
Expected: 构建成功

**Step 8: Commit**

```bash
git add plugins/db-doc/frontend/src/App.tsx plugins/db-doc/frontend/src/App.css
git commit -m "feat(db-doc): add export configuration panel with format/template selection"
```

---

## Task 10: 统一错误处理

**Files:**
- Modify: `plugins/db-doc/frontend/src/App.tsx`

**Step 1: 添加统一的 API 调用包装函数**

```typescript
const callAPI = async <T,>(method: string, params?: Record<string, unknown>): Promise<T> => {
  try {
    setError(null)
    return await window.pluginAPI.call('db-doc', method, params) as T
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e)
    setError(message)
    showToast('error', message)
    throw e
  }
}
```

**Step 2: 用 callAPI 替换现有的 window.pluginAPI.call 调用**

- `loadConnections`: `callAPI<ConnectionConfig[]>('list_connections', {})`
- `loadTables`: `callAPI<string[]>('list_tables', { connection_id: connectionId })`
- 其他调用点类似替换

**Step 3: 移除 error 和 loading 的分散管理**

统一将 `setLoading(true/false)` 和 `setError(...)` 通过 `callAPI` 处理，移除各处重复的 try/catch。

**Step 4: 验证构建**

Run: `cd plugins/db-doc/frontend && npm run build`
Expected: 构建成功

**Step 5: Commit**

```bash
git add plugins/db-doc/frontend/src/App.tsx
git commit -m "refactor(db-doc): unify API error handling with callAPI wrapper"
```

---

## Task 11: Word 导出器实现

**Files:**
- Create: `plugins/db-doc/src/exporter/word.rs`
- Modify: `plugins/db-doc/src/exporter/mod.rs`
- Modify: `plugins/db-doc/src/lib.rs` (启用 Word 导出分支)

**Step 1: 创建 word.rs**

实现 `DocumentExporter` trait，使用 quick-xml + zip 构建 DOCX：

核心结构：
1. 创建 ZIP 文件
2. 写入 `[Content_Types].xml` — 声明内容类型
3. 写入 `_rels/.rels` — 包关系
4. 写入 `word/document.xml` — 主文档内容（标题 + 表格）
5. 写入 `word/_rels/document.xml.rels` — 文档关系

Simple 模板：3 列 Word 表格（字段名、类型、说明）
Detailed 模板：6 列 Word 表格 + 索引信息段落

OOXML 表格结构（每个表）：
```xml
<w:p><w:r><w:t>表名: users</w:t></w:r></w:p>
<w:p><w:r><w:t>表注释: 用户表</w:t></w:r></w:p>
<w:tbl>
  <w:tblPr><w:tblBorders>...</w:tblBorders></w:tblPr>
  <w:tr><!-- 表头行 -->
    <w:tc><w:p><w:r><w:t>字段名</w:t></w:r></w:p></w:tc>
    <w:tc><w:p><w:r><w:t>类型</w:t></w:r></w:p></w:tc>
    ...
  </w:tr>
  <w:tr><!-- 数据行 -->
    <w:tc><w:p><w:r><w:t>id</w:t></w:r></w:p></w:tc>
    ...
  </w:tr>
</w:tbl>
```

**Step 2: 在 mod.rs 中注册模块**

```rust
mod markdown;
mod word;

pub use markdown::MarkdownExporter;
pub use word::WordExporter;
```

以及导出 `DocumentExporter`。

**Step 3: 在 lib.rs 中启用 Word 导出分支**

```rust
ExportFormat::Word => {
    let exporter = exporter::WordExporter::new(config.template);
    exporter.export(&tables_info, &config)?
}
```

**Step 4: 写单元测试**

在 `word.rs` 中添加测试，用模拟数据验证 DOCX 文件生成：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{TableInfo, ColumnInfo, IndexInfo, ExportConfig};

    fn create_test_table() -> TableInfo { /* 复用 markdown 的测试数据 */ }

    #[test]
    fn test_export_word_simple() {
        let exporter = WordExporter::new(TemplateStyle::Simple);
        let tables = vec![create_test_table()];
        let config = ExportConfig {
            connection_id: "test".into(),
            tables: vec!["users".into()],
            output_dir: std::env::temp_dir().to_string_lossy().to_string(),
            format: ExportFormat::Word,
            template: TemplateStyle::Simple,
        };
        let files = exporter.export(&tables, &config).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with(".docx"));
        // 清理
        std::fs::remove_file(&files[0]).ok();
    }
}
```

**Step 5: 运行测试**

Run: `cargo test -p db-doc`
Expected: PASS

**Step 6: Commit**

```bash
git add plugins/db-doc/src/exporter/word.rs plugins/db-doc/src/exporter/mod.rs plugins/db-doc/src/lib.rs
git commit -m "feat(db-doc): implement Word (DOCX) export with quick-xml and zip"
```

---

## Task 12: PDF 导出器实现

**Files:**
- Create: `plugins/db-doc/src/exporter/pdf.rs`
- Modify: `plugins/db-doc/src/exporter/mod.rs`
- Modify: `plugins/db-doc/src/lib.rs` (启用 PDF 导出分支)

**Step 1: 创建 pdf.rs**

实现 `DocumentExporter` trait，使用 printpdf 绘制：

核心流程：
1. 创建 PdfDocument
2. 嵌入系统字体（macOS: `/System/Library/Fonts/PingFang.ttc` 或 `/System/Library/Fonts/STHeiti Light.ttc`）
3. 为每个表绘制标题 + 表格
4. 表格使用线条绘制边框
5. 自动分页

关键注意：printpdf 绘制需要手动计算坐标，每行高度约 20pt，页边距 40pt。

Simple 模板：3 列表格
Detailed 模板：6 列表格 + 索引区域

**Step 2: 在 mod.rs 中注册**

```rust
mod pdf;
pub use pdf::PdfExporter;
```

**Step 3: 在 lib.rs 中启用 PDF 导出**

```rust
ExportFormat::Pdf => {
    let exporter = exporter::PdfExporter::new(config.template);
    exporter.export(&tables_info, &config)?
}
```

**Step 4: 写单元测试**

测试 PDF 生成并验证文件不为空：

```rust
#[test]
fn test_export_pdf_detailed() {
    let exporter = PdfExporter::new(TemplateStyle::Detailed);
    let tables = vec![create_test_table()];
    let config = ExportConfig { /* ... */ };
    let files = exporter.export(&tables, &config).unwrap();
    assert!(files[0].ends_with(".pdf"));
    let metadata = std::fs::metadata(&files[0]).unwrap();
    assert!(metadata.len() > 0);
    std::fs::remove_file(&files[0]).ok();
}
```

**Step 5: 运行测试**

Run: `cargo test -p db-doc`
Expected: PASS

**Step 6: Commit**

```bash
git add plugins/db-doc/src/exporter/pdf.rs plugins/db-doc/src/exporter/mod.rs plugins/db-doc/src/lib.rs
git commit -m "feat(db-doc): implement PDF export with printpdf and system font embedding"
```

---

## Task 13: 端到端验证和构建

**Files:**
- Modify: `plugins/db-doc/assets/` (构建产物更新)

**Step 1: 前端完整构建**

Run: `cd plugins/db-doc/frontend && npm run build`
Expected: 构建成功，assets 目录更新

**Step 2: Rust 完整构建**

Run: `cd plugins/db-doc && cargo build --release`
Expected: 编译成功

**Step 3: 运行全部测试**

Run: `cargo test -p db-doc`
Expected: 所有测试 PASS

**Step 4: Commit**

```bash
git add plugins/db-doc/assets/ plugins/db-doc/
git commit -m "chore(db-doc): rebuild assets and verify full build"
```

---

## 实施注意事项

1. **Task 3 是关键依赖** — 文件对话框 API 必须先暴露，后续的导出面板才能工作
2. **Task 4-10 可以并行** — CSS 变量、Toast、步骤导航等互不依赖，可以由不同 subagent 同时实施
3. **Task 11-12 较复杂** — Word/PDF 导出器是全新的 Rust 代码，建议逐个实现
4. **前端没有热更新** — 每次修改 TSX 后需要 `npm run build` 才能在 Tauri 中看到效果
5. **插件前端无法直接访问 Tauri API** — 所有 Tauri 功能必须通过 PluginPlaceholder 注入的 pluginAPI 调用
