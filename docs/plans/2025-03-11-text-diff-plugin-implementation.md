# 文本比对插件实施计划

**基于**: [2025-03-11-text-diff-plugin-design.md](./2025-03-11-text-diff-plugin-design.md)
**创建日期**: 2025-03-11
**预计工期**: 5-7 天

## 目录

1. [阶段一: 基础架构](#阶段一-基础架构)
2. [阶段二: 核心功能](#阶段二-核心功能)
3. [阶段三: 增强功能](#阶段三-增强功能)
4. [阶段四: 测试和打包](#阶段四-测试和打包)

---

## 阶段一: 基础架构

**目标**: 搭建插件项目骨架,配置开发环境

### 1.1 创建插件项目结构

**文件**: `plugins/text-diff/`

```bash
# 创建目录结构
mkdir -p plugins/text-diff/{src,frontend,assets}
cd plugins/text-diff

# 初始化 Rust 项目
cargo init --lib
```

**任务清单**:

- [ ] 创建 `plugins/text-diff/` 目录
- [ ] 初始化 Cargo lib 项目
- [ ] 创建 `frontend/` 子目录
- [ ] 创建 `assets/` 子目录

### 1.2 配置 Cargo.toml

**文件**: `plugins/text-diff/Cargo.toml`

```toml
[package]
name = "text-diff"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
worktools-plugin-api = { path = "../../shared/plugin-api" }
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
similar = "2.4"  # diff 算法库
```

**任务清单**:

- [ ] 创建 `Cargo.toml` 文件
- [ ] 配置 cdylib 编译目标
- [ ] 添加依赖项

### 1.3 创建 manifest.json

**文件**: `plugins/text-diff/manifest.json`

```json
{
  "id": "text-diff",
  "name": "文本比对",
  "description": "实时文本比对工具",
  "version": "1.0.0",
  "icon": "🔍",
  "author": "Work Tools Team",
  "homepage": "https://github.com/worktools/text-diff",
  "files": {
    "macos": "libtext_diff.dylib",
    "linux": "libtext_diff.so",
    "windows": "text_diff.dll"
  },
  "assets": {
    "entry": "index.html"
  },
  "permissions": [
    "filesystem",
    "clipboard"
  ]
}
```

**任务清单**:

- [ ] 创建 `manifest.json` 文件
- [ ] 配置插件元数据
- [ ] 配置平台特定文件名
- [ ] 配置权限声明

### 1.4 实现基础 Rust 插件结构

**文件**: `plugins/text-diff/src/lib.rs`

```rust
use anyhow::Result;
use serde_json::Value;
use worktools_plugin_api::Plugin;

pub struct TextDiff;

impl Plugin for TextDiff {
    fn id(&self) -> &str {
        "text-diff"
    }

    fn name(&self) -> &str {
        "文本比对"
    }

    fn description(&self) -> &str {
        "实时文本比对工具"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn icon(&self) -> &str {
        "🔍"
    }

    fn get_view(&self) -> String {
        "<div>插件前端资源加载中...</div>".to_string()
    }

    fn handle_call(
        &mut self,
        method: &str,
        _params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            _ => Err(format!("未知方法: {}", method).into()),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(TextDiff));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
```

**任务清单**:

- [ ] 实现 `Plugin trait`
- [ ] 实现 `plugin_create` 导出函数
- [ ] 添加基础错误处理
- [ ] 测试编译: `cargo build --release`

### 1.5 初始化前端项目

**文件**: `plugins/text-diff/frontend/`

```bash
cd frontend

# 使用 Vite 创建 React + TypeScript 项目
npm create vite@latest . -- --template react-ts

# 安装依赖
npm install

# 安装 Monaco Editor
npm install monaco-editor
```

**任务清单**:

- [ ] 初始化 Vite + React + TypeScript 项目
- [ ] 安装 monaco-editor 依赖
- [ ] 配置 TypeScript 编译选项
- [ ] 测试开发服务器: `npm run dev`

### 1.6 配置 Vite 构建

**文件**: `plugins/text-diff/frontend/vite.config.ts`

```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: '../assets',
    emptyOutDir: true,
    rollupOptions: {
      output: {
        entryFileNames: 'main.js',
        chunkFileNames: 'main.js',
        assetFileNames: (assetInfo) => {
          if (assetInfo.name === 'index.html') return 'index.html';
          if (assetInfo.name?.endsWith('.css')) return 'styles.css';
          return 'main.js';
        }
      }
    }
  }
});
```

**任务清单**:

- [ ] 修改 `vite.config.ts` 输出路径
- [ ] 配置资源文件命名
- [ ] 测试构建: `npm run build`
- [ ] 验证 `assets/` 目录生成正确文件

**阶段一验收标准**:
- ✅ 插件可以成功编译为动态库
- ✅ 前端项目可以独立运行和构建
- ✅ 构建产物输出到 `assets/` 目录
- ✅ 可以通过插件商店加载 (显示"插件前端资源加载中...")

---

## 阶段二: 核心功能

**目标**: 实现基础的文本比对功能

### 2.1 实现 Rust 后端 API

**文件**: `plugins/text-diff/src/lib.rs`

#### 2.1.1 添加数据结构

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextFileContent {
    pub content: String,
    pub encoding: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessOptions {
    pub ignore_whitespace: bool,
    pub ignore_case: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
    pub modifications: usize,
}
```

**任务清单**:

- [ ] 添加 `TextFileContent` 结构
- [ ] 添加 `ProcessOptions` 结构
- [ ] 添加 `DiffStats` 结构

#### 2.1.2 实现 load_text_file

```rust
fn load_text_file(file_path: &str) -> Result<TextFileContent> {
    use std::path::Path;

    // 验证文件是否存在
    if !Path::new(file_path).exists() {
        return Ok(serde_json::json!({
            "error": "文件不存在",
            "code": "FILE_NOT_FOUND"
        }).into());
    }

    // 验证文件大小 (限制 10MB)
    let metadata = std::fs::metadata(file_path)?;
    if metadata.len() > 10 * 1024 * 1024 {
        return Ok(serde_json::json!({
            "error": "文件过大 (最大 10MB)",
            "code": "FILE_TOO_LARGE"
        }).into());
    }

    // 读取文件内容
    let content = std::fs::read_to_string(file_path)?;

    Ok(TextFileContent {
        content,
        encoding: "utf-8".to_string()
    })
}
```

**任务清单**:

- [ ] 实现 `load_text_file` 函数
- [ ] 添加文件存在性检查
- [ ] 添加文件大小限制
- [ ] 在 `handle_call` 中路由到该方法

#### 2.1.3 实现 save_text_file

```rust
fn save_text_file(file_path: &str, content: &str) -> Result<()> {
    std::fs::write(file_path, content)?;
    Ok(())
}
```

**任务清单**:

- [ ] 实现 `save_text_file` 函数
- [ ] 在 `handle_call` 中路由到该方法

#### 2.1.4 实现 preprocess_text

```rust
fn preprocess_text(text: &str, options: &ProcessOptions) -> String {
    let mut result = text.to_string();

    if options.ignore_case {
        result = result.to_lowercase();
    }

    if options.ignore_whitespace {
        result = result.lines()
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join("\n");
    }

    result
}
```

**任务清单**:

- [ ] 实现 `preprocess_text` 函数
- [ ] 实现忽略大小写逻辑
- [ ] 实现忽略空白逻辑
- [ ] 在 `handle_call` 中路由到该方法

#### 2.1.5 实现 count_diff

```rust
fn count_diff_lines(original: &str, modified: &str) -> DiffStats {
    use similar::{Algorithm, ChangeTag, TextDiff};

    let diff = TextDiff::configure()
        .algorithm(Algorithm::Patience)
        .diff_lines(original, modified);

    let mut additions = 0;
    let mut deletions = 0;
    let mut modifications = 0;

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Insert => additions += 1,
            ChangeTag::Delete => deletions += 1,
            _ => {}
        }
    }

    modifications = additions.min(deletions);
    additions -= modifications;
    deletions -= modifications;

    DiffStats { additions, deletions, modifications }
}
```

**任务清单**:

- [ ] 实现 `count_diff_lines` 函数
- [ ] 使用 `similar` crate 计算差异
- [ ] 在 `handle_call` 中路由到该方法

#### 2.1.6 实现 export_diff

```rust
fn export_unified_diff(original: &str, modified: &str, filename: &str) -> String {
    use similar::{Algorithm, TextDiff};
    use std::io::Write;

    let diff = TextDiff::configure()
        .algorithm(Algorithm::Patience)
        .diff_lines(original, modified);

    let mut output = Vec::new();
    writeln!(&mut output, "--- a/{}", filename).ok();
    writeln!(&mut output, "+++ b/{}", filename).ok();

    // 使用 similar 的 Unified Diff 格式化器
    // (简化实现,实际需要使用 similar 的 UnifiedDiff 输出)

    String::from_utf8_lossy(&output).to_string()
}
```

**任务清单**:

- [ ] 实现 `export_unified_diff` 函数
- [ ] 生成标准 Unified Diff 格式
- [ ] 在 `handle_call` 中路由到该方法

### 2.2 实现前端 DiffEditor 组件

**文件**: `plugins/text-diff/frontend/src/DiffEditor.tsx`

```typescript
import { useEffect, useRef, useCallback } from 'react';
import * as monaco from 'monaco-editor';

interface DiffEditorProps {
  originalText: string;
  modifiedText: string;
  options: {
    ignoreWhitespace: boolean;
    ignoreCase: boolean;
  };
  onEditorReady?: (editor: monaco.editor.IStandaloneDiffEditor) => void;
}

export function DiffEditor({ originalText, modifiedText, options, onEditorReady }: DiffEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<monaco.editor.IStandaloneDiffEditor | null>(null);

  // 初始化 Monaco Diff Editor
  useEffect(() => {
    if (!containerRef.current) return;

    editorRef.current = monaco.editor.createDiffEditor(containerRef.current, {
      enableSplitViewResizing: true,
      renderSideBySide: true,
      ignoreTrimWhitespace: options.ignoreWhitespace,
      readOnly: false,
      automaticLayout: true,
      theme: 'vs-dark'
    });

    onEditorReady?.(editorRef.current);

    return () => {
      editorRef.current?.dispose();
    };
  }, []);

  // 更新文本模型
  useEffect(() => {
    if (!editorRef.current) return;

    const originalModel = monaco.editor.createModel(originalText, 'plaintext');
    const modifiedModel = monaco.editor.createModel(modifiedText, 'plaintext');

    editorRef.current.setModel({
      original: originalModel,
      modified: modifiedModel
    });

    return () => {
      originalModel.dispose();
      modifiedModel.dispose();
    };
  }, [originalText, modifiedText]);

  return <div ref={containerRef} className="diff-editor-container" />;
}
```

**任务清单**:

- [ ] 创建 `DiffEditor.tsx` 组件
- [ ] 实现 Monaco Diff Editor 初始化
- [ ] 实现文本模型更新
- [ ] 添加清理逻辑 (dispose)
- [ ] 暴露 editor 引用给父组件

### 2.3 实现前端 Toolbar 组件

**文件**: `plugins/text-diff/frontend/src/Toolbar.tsx`

```typescript
import { useState, useCallback } from 'react';

interface ToolbarProps {
  onOpenLeft: () => void;
  onOpenRight: () => void;
  onNextDiff: () => void;
  onPreviousDiff: () => void;
  onExport: () => void;
  onToggleIgnoreWhitespace: (value: boolean) => void;
  onToggleIgnoreCase: (value: boolean) => void;
  diffStats: {
    additions: number;
    deletions: number;
    modifications: number;
  };
}

export function Toolbar({
  onOpenLeft,
  onOpenRight,
  onNextDiff,
  onPreviousDiff,
  onExport,
  onToggleIgnoreWhitespace,
  onToggleIgnoreCase,
  diffStats
}: ToolbarProps) {
  const [ignoreWhitespace, setIgnoreWhitespace] = useState(false);
  const [ignoreCase, setIgnoreCase] = useState(false);

  const handleWhitespaceChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.checked;
    setIgnoreWhitespace(value);
    onToggleIgnoreWhitespace(value);
  }, [onToggleIgnoreWhitespace]);

  const handleCaseChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.checked;
    setIgnoreCase(value);
    onToggleIgnoreCase(value);
  }, [onToggleIgnoreCase]);

  return (
    <div className="toolbar">
      <button onClick={onOpenLeft}>📂 打开左侧文件</button>
      <button onClick={onOpenRight}>📂 打开右侧文件</button>

      <div className="separator" />

      <button onClick={onPreviousDiff}>↑ 上一个</button>
      <button onClick={onNextDiff}>↓ 下一个</button>

      <div className="separator" />

      <label>
        <input
          type="checkbox"
          checked={ignoreWhitespace}
          onChange={handleWhitespaceChange}
        />
        忽略空白
      </label>

      <label>
        <input
          type="checkbox"
          checked={ignoreCase}
          onChange={handleCaseChange}
        />
        忽略大小写
      </label>

      <div className="separator" />

      <button onClick={onExport}>💾 导出差异</button>

      <div className="stats">
        ➕ {diffStats.additions} | ➖ {diffStats.deletions} | ✏️ {diffStats.modifications}
      </div>
    </div>
  );
}
```

**任务清单**:

- [ ] 创建 `Toolbar.tsx` 组件
- [ ] 实现所有按钮事件处理
- [ ] 实现选项切换逻辑
- [ ] 显示差异统计信息
- [ ] 添加工具提示 (title 属性)

### 2.4 实现主应用组件

**文件**: `plugins/text-diff/frontend/src/App.tsx`

```typescript
import { useState, useCallback, useEffect, useRef } from 'react';
import { DiffEditor } from './DiffEditor';
import { Toolbar } from './Toolbar';
import './App.css';

// 声明 Tauri API
declare global {
  interface Window {
    pluginAPI: {
      call: (pluginId: string, method: string, params: any) => Promise<any>;
    };
  }
}

const FILE_FILTERS = [
  {
    name: 'All Text Files',
    extensions: ['txt', 'md', 'js', 'ts', 'jsx', 'tsx', 'json', 'html', 'css', 'scss']
  }
];

function App() {
  const [originalText, setOriginalText] = useState('');
  const [modifiedText, setModifiedText] = useState('');
  const [options, setOptions] = useState({
    ignoreWhitespace: false,
    ignoreCase: false
  });
  const [diffStats, setDiffStats] = useState({
    additions: 0,
    deletions: 0,
    modifications: 0
  });
  const [error, setError] = useState<string | null>(null);
  const editorRef = useRef<monaco.editor.IStandaloneDiffEditor | null>(null);

  // 文件打开处理
  const handleFileOpen = useCallback(async (side: 'left' | 'right') => {
    setError(null);

    try {
      // 调用 Tauri 文件选择器 (需要集成)
      const result = await window.pluginAPI.call('text-diff', 'load_text_file', {
        file_path: '/tmp/test.txt' // 临时测试路径
      });

      if (side === 'left') {
        setOriginalText(result.content);
      } else {
        setModifiedText(result.content);
      }
    } catch (err: any) {
      setError(err.message);
    }
  }, []);

  // 差异导航
  const handleNextDiff = useCallback(() => {
    if (!editorRef.current) return;
    editorRef.current.goToDiff('next');
  }, []);

  const handlePreviousDiff = useCallback(() => {
    if (!editorRef.current) return;
    editorRef.current.goToDiff('previous');
  }, []);

  // 导出差异
  const handleExport = useCallback(async () => {
    try {
      const result = await window.pluginAPI.call('text-diff', 'export_diff', {
        original: originalText,
        modified: modifiedText,
        filename: 'changes.diff'
      });

      // 下载文件 (需要实现)
      console.log('Exported diff:', result.diff);
    } catch (err: any) {
      setError(err.message);
    }
  }, [originalText, modifiedText]);

  // 选项切换
  const handleToggleIgnoreWhitespace = useCallback((value: boolean) => {
    setOptions(prev => ({ ...prev, ignoreWhitespace: value }));
  }, []);

  const handleToggleIgnoreCase = useCallback((value: boolean) => {
    setOptions(prev => ({ ...prev, ignoreCase: value }));
  }, []);

  // 计算差异统计
  useEffect(() => {
    if (!originalText || !modifiedText) return;

    window.pluginAPI.call('text-diff', 'count_diff', {
      original: originalText,
      modified: modifiedText
    }).then((stats: any) => {
      setDiffStats(stats);
    }).catch((err: any) => {
      console.error('Count diff error:', err);
    });
  }, [originalText, modifiedText]);

  return (
    <div className="app">
      {error && <div className="error">{error}</div>}

      <Toolbar
        onOpenLeft={() => handleFileOpen('left')}
        onOpenRight={() => handleFileOpen('right')}
        onNextDiff={handleNextDiff}
        onPreviousDiff={handlePreviousDiff}
        onExport={handleExport}
        onToggleIgnoreWhitespace={handleToggleIgnoreWhitespace}
        onToggleIgnoreCase={handleToggleIgnoreCase}
        diffStats={diffStats}
      />

      <DiffEditor
        originalText={originalText}
        modifiedText={modifiedText}
        options={options}
        onEditorReady={(editor) => {
          editorRef.current = editor;
        }}
      />
    </div>
  );
}

export default App;
```

**任务清单**:

- [ ] 创建 `App.tsx` 组件
- [ ] 实现状态管理
- [ ] 实现所有事件处理函数
- [ ] 集成 Toolbar 和 DiffEditor
- [ ] 添加错误显示

### 2.5 添加样式

**文件**: `plugins/text-diff/frontend/src/App.css`

```css
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

.app {
  width: 100%;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #1e1e1e;
}

.error {
  padding: 12px 16px;
  background: #d32f2f;
  color: white;
  font-size: 14px;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: #252526;
  border-bottom: 1px solid #3e3e42;
}

.toolbar button {
  padding: 6px 12px;
  background: #0078d4;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
  white-space: nowrap;
}

.toolbar button:hover {
  background: #005a9e;
}

.toolbar button:active {
  transform: scale(0.98);
}

.toolbar .separator {
  width: 1px;
  height: 24px;
  background: #3e3e42;
}

.toolbar label {
  display: flex;
  align-items: center;
  gap: 6px;
  color: #cccccc;
  font-size: 14px;
  cursor: pointer;
  user-select: none;
}

.toolbar label input[type="checkbox"] {
  cursor: pointer;
}

.toolbar .stats {
  margin-left: auto;
  color: #cccccc;
  font-size: 13px;
  white-space: nowrap;
}

.diff-editor-container {
  flex: 1;
  width: 100%;
  overflow: hidden;
}
```

**任务清单**:

- [ ] 创建 `App.css` 文件
- [ ] 实现工具栏样式
- [ ] 实现 Diff Editor 容器样式
- [ ] 添加错误提示样式

**阶段二验收标准**:
- ✅ 可以在左右两侧输入文本
- ✅ 实时显示差异高亮
- ✅ 可以点击按钮导航差异
- ✅ 显示差异统计信息
- ✅ 可以切换比对选项

---

## 阶段三: 增强功能

**目标**: 添加文件操作、快捷键、错误处理

### 3.1 集成 Tauri 文件对话框

**文件**: `plugins/text-diff/frontend/src/App.tsx`

```typescript
import { open, save } from '@tauri-apps/api/dialog';

// 修改 handleFileOpen 函数
const handleFileOpen = useCallback(async (side: 'left' | 'right') => {
  setError(null);

  try {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: 'All Text Files',
          extensions: ['txt', 'md', 'js', 'ts', 'jsx', 'tsx', 'json', 'html', 'css', 'scss']
        }
      ]
    });

    if (!selected || typeof selected !== 'string') return;

    const result = await window.pluginAPI.call('text-diff', 'load_text_file', {
      file_path: selected
    });

    if (side === 'left') {
      setOriginalText(result.content);
    } else {
      setModifiedText(result.content);
    }
  } catch (err: any) {
    setError(`加载文件失败: ${err.message}`);
  }
}, []);
```

**任务清单**:

- [ ] 安装 `@tauri-apps/api` 依赖
- [ ] 集成 `open` 文件对话框
- [ ] 集成 `save` 文件对话框
- [ ] 测试文件加载功能

### 3.2 实现键盘快捷键

**文件**: `plugins/text-diff/frontend/src/App.tsx`

```typescript
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    // F8: 下一个差异
    if (e.key === 'F8' && !e.shiftKey) {
      e.preventDefault();
      handleNextDiff();
    }
    // Shift + F8: 上一个差异
    if (e.key === 'F8' && e.shiftKey) {
      e.preventDefault();
      handlePreviousDiff();
    }
    // Ctrl+O: 打开左侧文件
    if ((e.ctrlKey || e.metaKey) && e.key === 'o') {
      e.preventDefault();
      handleFileOpen('left');
    }
  };

  window.addEventListener('keydown', handleKeyDown);
  return () => window.removeEventListener('keydown', handleKeyDown);
}, [handleNextDiff, handlePreviousDiff, handleFileOpen]);
```

**任务清单**:

- [ ] 实现 F8 快捷键
- [ ] 实现 Shift+F8 快捷键
- [ ] 实现 Ctrl+O 快捷键
- [ ] 添加快捷键提示到按钮 title

### 3.3 添加加载状态

**文件**: `plugins/text-diff/frontend/src/Toolbar.tsx`

```typescript
const [isLoading, setIsLoading] = useState(false);

// 在按钮上显示加载状态
<button onClick={onOpenLeft} disabled={isLoading}>
  {isLoading ? '⏳ 加载中...' : '📂 打开左侧文件'}
</button>
```

**任务清单**:

- [ ] 添加 loading 状态
- [ ] 禁用加载中的按钮
- [ ] 添加 loading 指示器
- [ ] 添加加载动画

### 3.4 实现错误边界

**文件**: `plugins/text-diff/frontend/src/main.tsx`

```typescript
import { ErrorBoundary } from 'react-error-boundary';

function ErrorFallback({ error }: { error: Error }) {
  return (
    <div className="error-fallback">
      <h2>❌ 出错了</h2>
      <p>{error.message}</p>
      <button onClick={() => window.location.reload()}>🔄 重新加载</button>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ErrorBoundary FallbackComponent={ErrorFallback}>
      <App />
    </ErrorBoundary>
  </React.StrictMode>
);
```

**任务清单**:

- [ ] 安装 `react-error-boundary`
- [ ] 实现 ErrorFallback 组件
- [ ] 包装 App 组件
- [ ] 测试错误边界

**阶段三验收标准**:
- ✅ 可以通过文件对话框打开文本文件
- ✅ 快捷键正常工作
- ✅ 加载状态正确显示
- ✅ 错误被正确捕获和显示

---

## 阶段四: 测试和打包

**目标**: 全面测试,准备发布

### 4.1 功能测试

**测试清单**:

- [ ] 测试空文本比对
- [ ] 测试大文件 (>1MB)
- [ ] 测试特殊字符 (emoji、换行符、Tab)
- [ ] 测试二进制文件 (应显示错误)
- [ ] 测试文件不存在
- [ ] 测试文件过大 (>10MB)
- [ ] 测试所有快捷键
- [ ] 测试所有比对选项
- [ ] 测试导出功能

### 4.2 性能测试

**测试场景**:

- [ ] 10KB 文件比对 (<100ms)
- [ ] 100KB 文件比对 (<500ms)
- [ ] 1MB 文件比对 (<2000ms)
- [ ] 内存使用 (<100MB)
- [ ] 编辑响应性 (无卡顿)

### 4.3 兼容性测试

**测试平台**:

- [ ] macOS (Intel)
- [ ] macOS (Apple Silicon)
- [ ] Windows 10/11
- [ ] Linux (Ubuntu)

### 4.4 构建插件包

**文件**: `plugins/text-diff/build.sh`

```bash
#!/bin/bash

set -e

echo "🔨 构建 text-diff 插件..."

# 构建 Rust 动态库
echo "📦 构建 Rust 动态库..."
cargo build --release

# 构建前端
echo "🌐 构建前端资源..."
cd frontend
npm run build
cd ..

# 打包为 .wtplugin.zip
echo "📦 打包插件包..."
zip -r text-diff.wtplugin.zip \
  manifest.json \
  target/release/libtext_diff.dylib \
  assets/

echo "✅ 构建完成: text-diff.wtplugin.zip"
```

**任务清单**:

- [ ] 创建构建脚本
- [ ] 测试构建流程
- [ ] 验证插件包内容
- [ ] 在主应用中测试导入

### 4.5 编写使用文档

**文件**: `plugins/text-diff/README.md`

```markdown
# 文本比对插件

## 功能特性

- 实时文本比对
- 差异高亮显示
- 文件导入/导出
- 键盘快捷键

## 使用方法

1. 点击"打开左侧文件"加载原始文本
2. 点击"打开右侧文件"加载修改后的文本
3. 查看差异高亮
4. 使用"上一个"/"下一个"按钮导航差异
5. 点击"导出差异"保存 Unified Diff 格式

## 快捷键

- `F8`: 下一个差异
- `Shift+F8`: 上一个差异
- `Ctrl+O`: 打开左侧文件

## 支持的文件格式

txt, md, js, ts, jsx, tsx, json, html, css, scss
```

**任务清单**:

- [ ] 编写 README.md
- [ ] 添加使用示例
- [ ] 添加快捷键说明
- [ ] 添加故障排除指南

**阶段四验收标准**:
- ✅ 所有功能测试通过
- ✅ 性能测试达标
- ✅ 可以成功构建插件包
- ✅ 可以在主应用中正常使用
- ✅ 文档完整清晰

---

## 总结

### 完成标准

- [ ] 所有 4 个阶段的任务完成
- [ ] 所有验收标准通过
- [ ] 插件包可以正常加载和使用
- [ ] 代码质量符合项目规范
- [ ] 文档完整

### 预计工期

- 阶段一: 1-2 天
- 阶段二: 2-3 天
- 阶段三: 1-2 天
- 阶段四: 1 天

**总计**: 5-7 天

### 风险和缓解措施

1. **Monaco Editor 集成复杂**
   - 风险: 可能遇到 API 兼容性问题
   - 缓解: 参考 VS Code 和 Monaco 文档,逐步集成

2. **性能问题**
   - 风险: 大文件可能导致性能问题
   - 缓解: 添加文件大小限制,使用防抖优化

3. **跨平台兼容性**
   - 风险: 不同平台可能表现不一致
   - 缓解: 在每个平台上进行充分测试

### 后续优化方向

- 支持 3-way merge
- 支持多文件批量比对
- 添加差异历史记录
- 集成 Git 仓库支持
