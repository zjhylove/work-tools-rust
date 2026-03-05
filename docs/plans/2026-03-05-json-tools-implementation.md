# JSON 工具插件实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 创建一个 JSON 工具插件,提供格式化、压缩、转义、去转义以及可视化编辑功能,采用双面板布局(左侧文本编辑 + 右侧树形视图)。

**Architecture:**
- 插件编译为动态库(.dylib/.so/.dll),通过 libloading 动态加载
- 前端使用 React + TypeScript,独立打包到插件目录
- 前端通过 `window.pluginAPI.call()` 与后端通信
- 临时工具模式,不持久化数据

**Tech Stack:**
- **后端**: Rust + serde_json + worktools-plugin-api
- **前端**: React 18 + TypeScript + Vite
- **样式**: CSS (复用密码管理器的样式变量)
- **图标**: { } (插件图标), ✨📦🔒🔑📂📁🗑️ (功能图标)

---

## Task 1: 创建插件项目结构

**Files:**
- Create: `plugins/json-tools/Cargo.toml`
- Create: `plugins/json-tools/src/lib.rs`
- Create: `plugins/json-tools/manifest.json`

**Step 1: 创建 Cargo.toml**

```toml
[package]
name = "json-tools"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
worktools-plugin-api = { path = "../../shared/plugin-api" }
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
```

**Step 2: 创建 manifest.json**

```json
{
  "id": "json-tools",
  "name": "JSON 工具",
  "description": "JSON 格式化、编辑和可视化工具",
  "version": "1.0.0",
  "icon": "{ }",
  "author": "Work Tools Team",
  "homepage": "https://github.com/worktools/json-tools",
  "files": {
    "macos": "libjson_tools.dylib",
    "linux": "libjson_tools.so",
    "windows": "json_tools.dll"
  },
  "assets": {
    "entry": "index.html"
  }
}
```

**Step 3: 创建基础 lib.rs**

```rust
use serde_json::Value;
use worktools_plugin_api::Plugin;
use anyhow::Result;

pub struct JsonTools;

impl Plugin for JsonTools {
    fn id(&self) -> &str { "json-tools" }
    fn name(&self) -> &str { "JSON 工具" }
    fn description(&self) -> &str { "JSON 格式化、编辑和可视化工具" }
    fn version(&self) -> &str { "1.0.0" }
    fn icon(&self) -> &str { "{ }" }
    fn get_view(&self) -> String {
        "<div>插件前端资源加载中...</div>".to_string()
    }

    fn handle_call(&mut self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            _ => Err(format!("未知方法: {method}").into()),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(JsonTools));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
```

**Step 4: 验证编译**

Run:
```bash
cd plugins/json-tools
cargo build
```

Expected: 编译成功,生成 `target/debug/libjson_tools.dylib`

**Step 5: Commit**

```bash
git add plugins/json-tools/
git commit -m "feat(json-tools): create plugin project structure"
```

---

## Task 2: 实现后端 JSON 处理方法

**Files:**
- Modify: `plugins/json-tools/src/lib.rs`

**Step 1: 实现 format_json 方法**

在 `handle_call` 方法的 match 语句中添加:

```rust
"format_json" => {
    let json_str = params.get("json")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("缺少 json 参数"))?;

    let parsed: Value = serde_json::from_str(json_str)?;
    let formatted = serde_json::to_string_pretty(&parsed)?;
    Ok(serde_json::json!({ "result": formatted }))
}
```

**Step 2: 实现 minify_json 方法**

```rust
"minify_json" => {
    let json_str = params.get("json")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("缺少 json 参数"))?;

    let parsed: Value = serde_json::from_str(json_str)?;
    let minified = serde_json::to_string(&parsed)?;
    Ok(serde_json::json!({ "result": minified }))
}
```

**Step 3: 实现 escape_json 方法**

```rust
"escape_json" => {
    let json_str = params.get("json")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("缺少 json 参数"))?;

    let escaped = json_str.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    Ok(serde_json::json!({ "result": escaped }))
}
```

**Step 4: 实现 unescape_json 方法**

```rust
"unescape_json" => {
    let json_str = params.get("json")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("缺少 json 参数"))?;

    let unescaped = json_str.replace("\\n", "\n")
        .replace("\\r", "\r")
        .replace("\\t", "\t")
        .replace("\\\"", "\"")
        .replace("\\\\", "\\");
    Ok(serde_json::json!({ "result": unescaped }))
}
```

**Step 5: 实现 validate_json 方法**

```rust
"validate_json" => {
    let json_str = params.get("json")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("缺少 json 参数"))?;

    match serde_json::from_str::<Value>(json_str) {
        Ok(_) => Ok(serde_json::json!({ "valid": true, "error": null })),
        Err(e) => {
            let error_msg = e.to_string();
            Ok(serde_json::json!({
                "valid": false,
                "error": error_msg
            }))
        }
    }
}
```

**Step 6: 验证编译**

Run:
```bash
cargo build
```

Expected: 编译成功,无警告

**Step 7: Commit**

```bash
git add plugins/json-tools/src/lib.rs
git commit -m "feat(json-tools): implement JSON processing methods"
```

---

## Task 3: 创建前端项目结构

**Files:**
- Create: `plugins/json-tools/frontend/package.json`
- Create: `plugins/json-tools/frontend/vite.config.ts`
- Create: `plugins/json-tools/frontend/tsconfig.json`
- Create: `plugins/json-tools/frontend/index.html`
- Create: `plugins/json-tools/frontend/src/main.tsx`
- Create: `plugins/json-tools/frontend/src/App.tsx`
- Create: `plugins/json-tools/frontend/src/App.css`

**Step 1: 创建 package.json**

```json
{
  "name": "json-tools-frontend",
  "version": "1.0.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0"
  },
  "devDependencies": {
    "@types/react": "^18.2.0",
    "@types/react-dom": "^18.2.0",
    "@vitejs/plugin-react": "^4.0.0",
    "typescript": "^5.0.0",
    "vite": "^4.3.0"
  }
}
```

**Step 2: 创建 vite.config.ts**

```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3000,
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
});
```

**Step 3: 创建 tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

**Step 4: 创建 tsconfig.node.json**

```json
{
  "compilerOptions": {
    "composite": true,
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowSyntheticDefaultImports": true
  },
  "include": ["vite.config.ts"]
}
```

**Step 5: 创建 index.html**

```html
<!DOCTYPE html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>JSON 工具</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

**Step 6: 创建 main.tsx**

```typescript
import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './App.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

**Step 7: 创建基础 App.tsx**

```typescript
import './App.css';

function App() {
  return (
    <div className="json-tools">
      <div className="loading">加载中...</div>
    </div>
  );
}

export default App;
```

**Step 8: 创建基础 App.css**

```css
.json-tools {
  flex: 1;
  padding: 24px;
  height: 100%;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
}
```

**Step 9: 安装依赖并验证**

Run:
```bash
cd plugins/json-tools/frontend
npm install
npm run dev
```

Expected: 开发服务器启动成功,访问 http://localhost:3000 显示"加载中..."

**Step 10: Commit**

```bash
git add plugins/json-tools/frontend/
git commit -m "feat(json-tools): create frontend project structure"
```

---

## Task 4: 实现前端工具函数

**Files:**
- Create: `plugins/json-tools/frontend/src/utils/jsonUtils.ts`
- Create: `plugins/json-tools/frontend/src/utils/treeUtils.ts`

**Step 1: 创建 jsonUtils.ts**

```typescript
export interface ValidationError {
  valid: boolean;
  error: string | null;
  line?: number;
  column?: number;
}

export function validateJson(jsonStr: string): ValidationError {
  try {
    JSON.parse(jsonStr);
    return { valid: true, error: null };
  } catch (e: any) {
    const errorStr = e.toString();
    const lineMatch = errorStr.match(/line (\d+)/);
    const columnMatch = errorStr.match(/column (\d+)/);

    return {
      valid: false,
      error: errorStr,
      line: lineMatch ? parseInt(lineMatch[1]) : undefined,
      column: columnMatch ? parseInt(columnMatch[1]) : undefined,
    };
  }
}

export function formatJson(jsonStr: string): string {
  const parsed = JSON.parse(jsonStr);
  return JSON.stringify(parsed, null, 2);
}

export function minifyJson(jsonStr: string): string {
  const parsed = JSON.parse(jsonStr);
  return JSON.stringify(parsed);
}

export function escapeJson(jsonStr: string): string {
  return jsonStr
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"')
    .replace(/\n/g, '\\n')
    .replace(/\r/g, '\\r')
    .replace(/\t/g, '\\t');
}

export function unescapeJson(jsonStr: string): string {
  return jsonStr
    .replace(/\\n/g, '\n')
    .replace(/\\r/g, '\r')
    .replace(/\\t/g, '\t')
    .replace(/\\"/g, '"')
    .replace(/\\\\/g, '\\');
}
```

**Step 2: 创建 treeUtils.ts**

```typescript
export type JsonPath = Array<string | number>;

export function getValueByPath(obj: any, path: JsonPath): any {
  return path.reduce((current, key) => current?.[key], obj);
}

export function deleteByPath(obj: any, path: JsonPath): any {
  if (path.length === 0) return obj;

  const [key, ...rest] = path;

  if (rest.length === 0) {
    if (Array.isArray(obj)) {
      return obj.filter((_, i) => i !== key);
    } else {
      const { [key]: _, ...result } = obj;
      return result;
    }
  }

  if (Array.isArray(obj)) {
    return obj.map((item, i) =>
      i === key ? deleteByPath(item, rest) : item
    );
  } else {
    return {
      ...obj,
      [key]: obj[key] !== undefined ? deleteByPath(obj[key], rest) : undefined
    };
  }
}

export function expandAll(data: any): Record<string, boolean> {
  const result: Record<string, boolean> = { 'root': true };

  function traverse(obj: any, path: string[]) {
    const pathStr = path.join('.');
    result[pathStr] = true;

    if (Array.isArray(obj)) {
      obj.forEach((item, i) => traverse(item, [...path, i]));
    } else if (typeof obj === 'object' && obj !== null) {
      Object.keys(obj).forEach(key => {
        traverse(obj[key], [...path, key]);
      });
    }
  }

  traverse(data, []);
  return result;
}
```

**Step 3: 验证 TypeScript 编译**

Run:
```bash
cd plugins/json-tools/frontend
npx tsc --noEmit
```

Expected: 无类型错误

**Step 4: Commit**

```bash
git add plugins/json-tools/frontend/src/utils/
git commit -m "feat(json-tools): implement utility functions"
```

---

## Task 5: 实现工具栏组件

**Files:**
- Create: `plugins/json-tools/frontend/src/components/Toolbar.tsx`

**Step 1: 创建 Toolbar.tsx**

```typescript
import React from 'react';

interface ToolbarProps {
  isValid: boolean;
  onAction: (action: string) => void;
}

export default function Toolbar({ isValid, onAction }: ToolbarProps) {
  const tools = [
    { id: 'format', label: '格式化', icon: '✨' },
    { id: 'minify', label: '压缩', icon: '📦' },
    { id: 'escape', label: '转义', icon: '🔒' },
    { id: 'unescape', label: '去转义', icon: '🔑' },
  ];

  const treeActions = [
    { id: 'expandAll', label: '全展开', icon: '📂' },
    { id: 'collapseAll', label: '全折叠', icon: '📁' },
    { id: 'deleteSelected', label: '删除选中', icon: '🗑️' },
  ];

  return (
    <div className="json-toolbar">
      <div className="toolbar-group">
        {tools.map(tool => (
          <button
            key={tool.id}
            className="btn-tool"
            disabled={!isValid}
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onAction(tool.id);
            }}
            title={tool.label}
          >
            {tool.icon} {tool.label}
          </button>
        ))}
      </div>

      <div className="toolbar-group">
        {treeActions.map(action => (
          <button
            key={action.id}
            className="btn-tool"
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onAction(action.id);
            }}
            title={action.label}
          >
            {action.icon} {action.label}
          </button>
        ))}
      </div>
    </div>
  );
}
```

**Step 2: 在 App.css 中添加工具栏样式**

```css
/* 工具栏 */
.json-toolbar {
  padding: 16px 20px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-color);
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.toolbar-group {
  display: flex;
  gap: 10px;
}

.btn-tool {
  padding: 9px 18px;
  background: var(--bg-primary);
  color: var(--text-primary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-tool:hover:not(:disabled) {
  background: var(--hover-bg);
  border-color: var(--accent);
  transform: translateY(-1px);
}

.btn-tool:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
```

**Step 3: Commit**

```bash
git add plugins/json-tools/frontend/src/components/Toolbar.tsx
git commit -m "feat(json-tools): implement toolbar component"
```

---

## Task 6: 实现文本编辑器组件

**Files:**
- Create: `plugins/json-tools/frontend/src/components/JsonEditor.tsx`

**Step 1: 创建 JsonEditor.tsx**

```typescript
import React from 'react';
import type { ValidationError } from '../utils/jsonUtils';

interface JsonEditorProps {
  value: string;
  onChange: (value: string) => void;
  error: ValidationError | null;
}

export default function JsonEditor({ value, onChange, error }: JsonEditorProps) {
  return (
    <div className="json-editor-panel">
      <textarea
        className="json-editor"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="在此输入或粘贴 JSON..."
        spellCheck={false}
      />
    </div>
  );
}
```

**Step 2: 在 App.css 中添加编辑器样式**

```css
/* 双面板容器 */
.json-workspace {
  display: flex;
  gap: 1px;
  flex: 1;
  overflow: hidden;
  background: var(--border-color);
}

.json-editor-panel,
.json-tree-panel {
  flex: 1;
  background: var(--bg-primary);
  overflow: auto;
}

/* 文本编辑器 */
.json-editor {
  width: 100%;
  height: 100%;
  border: none;
  padding: 20px;
  font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
  font-size: 13px;
  line-height: 1.6;
  resize: none;
  background: var(--bg-primary);
  color: var(--text-primary);
}

.json-editor:focus {
  outline: none;
}
```

**Step 3: Commit**

```bash
git add plugins/json-tools/frontend/src/components/JsonEditor.tsx
git commit -m "feat(json-tools): implement JSON editor component"
```

---

## Task 7: 实现树形视图组件

**Files:**
- Create: `plugins/json-tools/frontend/src/components/JsonTree.tsx`

**Step 1: 创建 JsonTree.tsx**

```typescript
import React from 'react';
import type { JsonPath } from '../utils/treeUtils';

interface JsonTreeProps {
  data: any;
  selectedPath: JsonPath | null;
  isExpanded: Record<string, boolean>;
  onSelectPath: (path: JsonPath) => void;
  onToggleExpand: (path: JsonPath) => void;
}

export default function JsonTree({ data, selectedPath, isExpanded, onSelectPath, onToggleExpand }: JsonTreeProps) {
  if (!data) {
    return (
      <div className="json-tree-panel">
        <div className="empty-state">
          <div className="empty-icon">📋</div>
          <div className="empty-text">输入 JSON 后在此显示树形视图</div>
        </div>
      </div>
    );
  }

  return (
    <div className="json-tree-panel">
      <div className="json-tree">
        <TreeNode
          data={data}
          path={[]}
          selectedPath={selectedPath}
          isExpanded={isExpanded}
          onSelectPath={onSelectPath}
          onToggleExpand={onToggleExpand}
        />
      </div>
    </div>
  );
}

interface TreeNodeProps {
  data: any;
  path: JsonPath;
  selectedPath: JsonPath | null;
  isExpanded: Record<string, boolean>;
  onSelectPath: (path: JsonPath) => void;
  onToggleExpand: (path: JsonPath) => void;
}

function TreeNode({ data, path, selectedPath, isExpanded, onSelectPath, onToggleExpand }: TreeNodeProps) {
  const pathStr = path.join('.');
  const isSelected = selectedPath !== null &&
    path.length === selectedPath.length &&
    path.every((p, i) => p === selectedPath[i]);

  const isContainer = Array.isArray(data) || (typeof data === 'object' && data !== null);
  const expanded = isExpanded[pathStr] ?? (path.length === 0);

  const handleClick = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    onSelectPath(path);
  };

  const handleToggle = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (isContainer) {
      onToggleExpand(path);
    }
  };

  if (!isContainer) {
    return (
      <div
        className={`tree-node ${isSelected ? 'selected' : ''}`}
        onClick={handleClick}
      >
        <ValueNode value={data} />
      </div>
    );
  }

  const keys = Object.keys(data);

  return (
    <div className="tree-node-container">
      <div
        className={`tree-node ${isSelected ? 'selected' : ''}`}
        onClick={handleClick}
      >
        <span
          className="tree-toggle"
          onClick={handleToggle}
        >
          {expanded ? '▼' : '▶'}
        </span>
        <span className="tree-key">
          {Array.isArray(data) ? `array[${keys.length}]` : `object{${keys.length}}`}
        </span>
      </div>

      {expanded && (
        <div className="tree-children">
          {keys.map((key, index) => {
            const childPath = [...path, Array.isArray(data) ? parseInt(key) : key];
            const childData = data[key];

            return (
              <div key={key} className="tree-child">
                {!Array.isArray(data) && (
                  <span className="tree-key">"{key}": </span>
                )}
                <TreeNode
                  data={childData}
                  path={childPath}
                  selectedPath={selectedPath}
                  isExpanded={isExpanded}
                  onSelectPath={onSelectPath}
                  onToggleExpand={onToggleExpand}
                />
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

function ValueNode({ value }: { value: any }) {
  if (value === null) {
    return <span className="tree-null">null</span>;
  }

  if (typeof value === 'string') {
    return <span className="tree-string">"{value}"</span>;
  }

  if (typeof value === 'number') {
    return <span className="tree-number">{value}</span>;
  }

  if (typeof value === 'boolean') {
    return <span className="tree-boolean">{value.toString()}</span>;
  }

  return <span>{String(value)}</span>;
}
```

**Step 2: 在 App.css 中添加树形视图样式**

```css
/* 树形视图 */
.json-tree {
  padding: 20px;
  font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
  font-size: 13px;
}

.tree-node {
  padding: 4px 0;
  padding-left: 20px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.15s;
}

.tree-node:hover {
  background: var(--hover-bg);
}

.tree-node.selected {
  background: var(--accent-light);
  border-left: 3px solid var(--accent);
}

.tree-node-container {
  margin-left: -20px;
}

.tree-children {
  margin-left: 20px;
}

.tree-child {
  display: flex;
  align-items: flex-start;
}

.tree-toggle {
  margin-right: 6px;
  cursor: pointer;
  user-select: none;
}

.tree-key {
  color: #d32f2f;
  font-weight: 600;
}

.tree-string {
  color: #2e7d32;
}

.tree-number {
  color: #1565c0;
}

.tree-boolean {
  color: #c62828;
}

.tree-null {
  color: #7f8c8d;
  font-style: italic;
}

/* 空状态 */
.empty-state {
  text-align: center;
  padding: 60px 20px;
  color: var(--text-tertiary);
}

.empty-icon {
  font-size: 56px;
  margin-bottom: 12px;
}

.empty-text {
  font-size: 14px;
}
```

**Step 3: Commit**

```bash
git add plugins/json-tools/frontend/src/components/JsonTree.tsx
git commit -m "feat(json-tools): implement JSON tree component"
```

---

## Task 8: 实现主应用组件

**Files:**
- Modify: `plugins/json-tools/frontend/src/App.tsx`

**Step 1: 实现 App.tsx**

```typescript
import { useState, useEffect } from 'react';
import './App.css';
import Toolbar from './components/Toolbar';
import JsonEditor from './components/JsonEditor';
import JsonTree from './components/JsonTree';
import { validateJson, formatJson, minifyJson, escapeJson, unescapeJson } from './utils/jsonUtils';
import { deleteByPath, expandAll, type JsonPath } from './utils/treeUtils';

interface ValidationError {
  valid: boolean;
  error: string | null;
  line?: number;
  column?: number;
}

function App() {
  const [jsonText, setJsonText] = useState<string>('{\n  \n}');
  const [parsedData, setParsedData] = useState<any>(null);
  const [error, setError] = useState<ValidationError | null>(null);
  const [selectedPath, setSelectedPath] = useState<JsonPath | null>(null);
  const [isExpanded, setIsExpanded] = useState<Record<string, boolean>>({ 'root': true });
  const [successMessage, setSuccessMessage] = useState<string>('');

  // 实时验证 JSON
  useEffect(() => {
    const validation = validateJson(jsonText);
    setError(validation);

    if (validation.valid) {
      try {
        const parsed = JSON.parse(jsonText);
        setParsedData(parsed);
      } catch (e) {
        // 解析失败,保持原状
      }
    } else {
      setParsedData(null);
    }
  }, [jsonText]);

  // 处理工具栏操作
  const handleToolAction = async (action: string) => {
    try {
      let result: string;

      switch (action) {
        case 'format':
          result = formatJson(jsonText);
          break;
        case 'minify':
          result = minifyJson(jsonText);
          break;
        case 'escape':
          result = escapeJson(jsonText);
          break;
        case 'unescape':
          result = unescapeJson(jsonText);
          break;
        case 'expandAll':
          setIsExpanded(expandAll(parsedData));
          return;
        case 'collapseAll':
          setIsExpanded({ 'root': true });
          return;
        case 'deleteSelected':
          if (selectedPath) {
            const newData = deleteByPath(parsedData, selectedPath);
            const newText = JSON.stringify(newData, null, 2);
            setParsedData(newData);
            setJsonText(newText);
            setSelectedPath(null);
          }
          return;
        default:
          return;
      }

      setJsonText(result);
      setSuccessMessage('操作成功');
      setTimeout(() => setSuccessMessage(''), 2000);
    } catch (e) {
      setError({
        valid: false,
        error: (e as Error).message
      });
    }
  };

  return (
    <div className="json-tools">
      <Toolbar
        isValid={error?.valid ?? false}
        onAction={handleToolAction}
      />

      {successMessage && (
        <div className="json-success">
          ✓ {successMessage}
        </div>
      )}

      <div className="json-workspace">
        <JsonEditor
          value={jsonText}
          onChange={setJsonText}
          error={error}
        />
        <JsonTree
          data={parsedData}
          selectedPath={selectedPath}
          isExpanded={isExpanded}
          onSelectPath={setSelectedPath}
          onToggleExpand={(path) => {
            const pathStr = path.join('.');
            setIsExpanded(prev => ({
              ...prev,
              [pathStr]: !prev[pathStr]
            }));
          }}
        />
      </div>

      {error && !error.valid && (
        <div className="json-error">
          ⚠️ {error.error}
        </div>
      )}
    </div>
  );
}

export default App;
```

**Step 2: 在 App.css 中添加提示消息样式**

```css
/* 成功提示 */
.json-success {
  padding: 12px 20px;
  background: #f0f9f4;
  border: 2px solid #4caf50;
  border-radius: 10px;
  color: #2e7d32;
  font-size: 14px;
  font-weight: 500;
  margin: 0 20px 16px 20px;
  display: flex;
  align-items: center;
  gap: 8px;
  animation: slideDown 0.3s ease;
}

/* 错误提示 */
.json-error {
  padding: 12px 20px;
  background: #fee;
  border: 2px solid #f88;
  border-radius: 10px;
  color: #c33;
  font-size: 14px;
  font-weight: 500;
  margin: 16px 20px 0 20px;
  display: flex;
  align-items: center;
  gap: 8px;
}

@keyframes slideDown {
  from {
    opacity: 0;
    transform: translateY(-20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
```

**Step 3: Commit**

```bash
git add plugins/json-tools/frontend/src/App.tsx
git commit -m "feat(json-tools): implement main App component"
```

---

## Task 9: 添加 CSS 变量定义

**Files:**
- Modify: `plugins/json-tools/frontend/src/App.css`

**Step 1: 在 App.css 顶部添加 CSS 变量**

```css
:root {
  --accent: #0078d4;
  --accent-light: rgba(0, 120, 212, 0.1);
  --accent-hover: #005a9e;
  --bg-primary: #ffffff;
  --bg-secondary: #f5f5f5;
  --bg-tertiary: #f0f0f0;
  --text-primary: #1e1e1e;
  --text-secondary: #7f8c8d;
  --text-tertiary: #999999;
  --border-color: #e0e0e0;
  --hover-bg: #f0f0f0;
  --shadow-sm: 0 2px 8px rgba(0, 0, 0, 0.08);
  --shadow-md: 0 4px 16px rgba(0, 0, 0, 0.12);
  --error-color: #dc3545;
}
```

**Step 2: Commit**

```bash
git add plugins/json-tools/frontend/src/App.css
git commit -m "feat(json-tools): add CSS variables"
```

---

## Task 10: 测试前端构建

**Files:**
- Build: `plugins/json-tools/frontend/dist/`

**Step 1: 构建前端**

Run:
```bash
cd plugins/json-tools/frontend
npm run build
```

Expected: 生成 `dist/` 目录,包含 `index.html` 和打包后的 JS/CSS 文件

**Step 2: 验证构建产物**

Run:
```bash
ls -la dist/
```

Expected: 看到 `index.html`, `assets/index.xxx.js`, `assets/index.xxx.css` 等文件

**Step 3: Commit**

```bash
git add plugins/json-tools/frontend/dist/
git commit -m "feat(json-tools): build frontend assets"
```

---

## Task 11: 构建并打包插件

**Files:**
- Build: `plugins/json-tools/target/release/libjson_tools.dylib`
- Package: `json-tools.wtplugin.zip`

**Step 1: 构建插件动态库**

Run:
```bash
cd plugins/json-tools
cargo build --release
```

Expected: 生成 `target/release/libjson_tools.dylib` (macOS)

**Step 2: 验证动态库**

Run:
```bash
nm -gU target/release/libjson_tools.dylib | grep plugin_create
```

Expected: 看到 `plugin_create` 符号

**Step 3: 打包插件**

Run:
```bash
cd plugins/json-tools
zip -r json-tools.wtplugin.zip \
  manifest.json \
  target/release/libjson_tools.dylib \
  frontend/dist/
```

Expected: 生成 `json-tools.wtplugin.zip` 文件

**Step 4: 验证插件包**

Run:
```bash
unzip -l json-tools.wtplugin.zip
```

Expected: 看到 `manifest.json`, `libjson_tools.dylib`, `dist/` 目录

**Step 5: Commit**

```bash
git add json-tools.wtplugin.zip
git commit -m "feat(json-tools): package plugin"
```

---

## Task 12: 安装并测试插件

**Files:**
- Install: `~/.worktools/plugins/json-tools/`

**Step 1: 安装插件**

Run:
```bash
mkdir -p ~/.worktools/plugins/json-tools
unzip json-tools.wtplugin.zip -d ~/.worktools/plugins/json-tools/
```

**Step 2: 启动应用测试**

Run:
```bash
cd ../../tauri-app
npm run tauri dev
```

Expected:
1. 应用启动后,侧边栏显示 JSON 工具插件 (图标: { })
2. 点击插件,显示双面板界面
3. 左侧工具栏: ✨格式化 📦压缩 🔒转义 🔑去转义
4. 右侧操作栏: 📂全展开 📁全折叠 🗑️删除选中
5. 输入 JSON 后,右侧显示树形视图

**Step 3: 功能测试清单**

- [ ] 输入有效 JSON,右侧显示树形视图
- [ ] 输入无效 JSON,底部显示错误提示,工具栏按钮禁用
- [ ] 点击"格式化",JSON 美化显示
- [ ] 点击"压缩",JSON 压缩为单行
- [ ] 点击"转义",特殊字符被转义
- [ ] 点击"去转义",转义字符被还原
- [ ] 点击树形节点,节点高亮显示
- [ ] 点击"删除选中",选中节点被删除
- [ ] 点击"全展开",所有节点展开
- [ ] 点击"全折叠",所有节点折叠
- [ ] 点击节点箭头,切换展开/折叠状态

**Step 4: 编写测试文档**

Create: `plugins/json-tools/TESTING.md`

```markdown
# JSON 工具插件测试文档

## 测试环境
- macOS 14.x / Windows 11 / Linux Ubuntu 22.04
- Node.js 18+
- Rust 1.70+

## 手动测试步骤

### 1. 基础功能测试
1. 启动应用,检查侧边栏是否显示 JSON 工具插件
2. 点击插件,检查界面是否正常显示
3. 输入有效 JSON,检查右侧树形视图是否正确显示

### 2. 工具栏功能测试
#### 格式化测试
- 输入: `{"name":"test","value":123}`
- 点击"格式化"
- 预期: JSON 美化为多行格式

#### 压缩测试
- 输入格式化的 JSON
- 点击"压缩"
- 预期: JSON 压缩为单行

#### 转义测试
- 输入: `{"text":"Hello\nWorld"}`
- 点击"转义"
- 预期: 换行符被转义为 `\n`

#### 去转义测试
- 输入转义后的 JSON
- 点击"去转义"
- 预期: 转义序列被还原

### 3. 树形视图测试
- 点击节点,检查是否高亮
- 点击箭头,检查是否展开/折叠
- 点击"全展开",检查所有节点是否展开
- 点击"全折叠",检查是否只保留根节点

### 4. 删除功能测试
- 选中一个节点
- 点击"删除选中"
- 预期: 节点被删除,左侧编辑器和右侧树形视图同步更新

### 5. 错误处理测试
- 输入无效 JSON: `{invalid}`
- 预期: 底部显示错误提示,工具栏按钮禁用
- 修正 JSON 后,错误提示消失,按钮恢复可用

## 已知问题
(记录测试中发现的问题)
```

**Step 5: Commit**

```bash
git add plugins/json-tools/TESTING.md
git commit -m "docs(json-tools): add testing documentation"
```

---

## Task 13: 编写 README 文档

**Files:**
- Create: `plugins/json-tools/README.md`

**Step 1: 创建 README.md**

```markdown
# JSON 工具插件

一个强大的 JSON 编辑和可视化工具,提供格式化、压缩、转义、去转义以及树形视图编辑功能。

## 功能特性

- ✨ **格式化**: 美化 JSON,提高可读性
- 📦 **压缩**: 压缩 JSON,减小文件大小
- 🔒 **转义**: 转义特殊字符,用于字符串嵌入
- 🔑 **去转义**: 还原转义序列
- 📂 **树形视图**: 可视化展示 JSON 结构
- 🗑️ **节点删除**: 选择并删除树形视图中的节点
- ⚡ **实时验证**: 即时检测 JSON 语法错误

## 截图

(添加使用截图)

## 安装方法

### 方式一: 插件包安装 (推荐)

1. 下载 `json-tools.wtplugin.zip`
2. 打开 Work Tools 应用
3. 点击插件商店按钮 (🧩)
4. 选择插件包文件导入

### 方式二: 手动安装

```bash
# 解压插件包到用户目录
mkdir -p ~/.worktools/plugins/json-tools
unzip json-tools.wtplugin.zip -d ~/.worktools/plugins/json-tools/

# 重启应用
```

## 使用方法

### 基础使用

1. 在左侧编辑器输入或粘贴 JSON
2. 右侧自动显示树形视图
3. 使用工具栏按钮进行各种操作

### 高级功能

#### 节点删除
1. 在右侧树形视图中点击选择节点
2. 点击"删除选中"按钮
3. 节点被删除,左侧编辑器自动更新

#### 展开/折叠
- 点击"全展开": 展开所有节点
- 点击"全折叠": 只保留根节点展开
- 点击节点箭头: 切换单个节点的展开/折叠状态

## 开发

### 环境要求

- Rust 1.70+
- Node.js 18+
- npm 或 yarn

### 构建步骤

```bash
# 克隆仓库
git clone https://github.com/worktools/json-tools.git
cd json-tools

# 构建后端
cargo build --release

# 构建前端
cd frontend
npm install
npm run build

# 打包插件
cd ..
zip -r json-tools.wtplugin.zip \
  manifest.json \
  target/release/libjson_tools.dylib \
  frontend/dist/
```

## 技术栈

- **后端**: Rust + serde_json
- **前端**: React 18 + TypeScript + Vite
- **样式**: CSS3
- **插件系统**: Work Tools Plugin API

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request!

## 作者

Work Tools Team
```

**Step 2: Commit**

```bash
git add plugins/json-tools/README.md
git commit -m "docs(json-tools): add README documentation"
```

---

## Task 14: 最终验证和清理

**Files:**
- Verify: 所有功能正常工作
- Clean: 移除临时文件和调试代码

**Step 1: 完整功能测试**

按照 Task 12 中的测试清单逐项验证

**Step 2: 检查代码质量**

Run:
```bash
cd plugins/json-tools
cargo clippy -- -D warnings
```

Expected: 无警告

**Step 3: 格式化代码**

Run:
```bash
cargo fmt
cd frontend
npx prettier --write "src/**/*.{ts,tsx,css}"
```

**Step 4: 检查构建产物**

Run:
```bash
cd plugins/json-tools
cargo build --release
cd frontend
npm run build
```

Expected: 构建成功,无错误

**Step 5: 最终打包**

Run:
```bash
cd plugins/json-tools
zip -r json-tools.wtplugin.zip \
  manifest.json \
  target/release/libjson_tools.dylib \
  frontend/dist/
```

**Step 6: 验证插件包大小**

Run:
```bash
ls -lh json-tools.wtplugin.zip
```

Expected: 文件大小合理 (< 5MB)

**Step 7: 最终提交**

```bash
git add -A
git commit -m "feat(json-tools): complete plugin implementation"
```

---

## Task 15: 文档更新和发布准备

**Files:**
- Update: `CLAUDE.md`
- Update: `README.md` (根目录)

**Step 1: 更新主项目 README**

在根目录 `README.md` 中添加:

```markdown
## 可用插件

### JSON 工具
JSON 格式化、压缩、转义和可视化编辑工具。

- **功能**: 格式化、压缩、转义、树形视图、节点删除
- **图标**: { }
- **版本**: 1.0.0
- **位置**: `plugins/json-tools/`
```

**Step 2: 更新 CLAUDE.md**

在 `CLAUDE.md` 的插件列表部分添加:

```markdown
### json-tools
JSON 工具插件,提供格式化、压缩、转义和可视化编辑功能。

**项目结构**:
```
plugins/json-tools/
├── src/lib.rs          # 插件实现
├── frontend/           # React 前端
│   ├── src/components/
│   │   ├── Toolbar.tsx      # 工具栏
│   │   ├── JsonEditor.tsx   # 文本编辑器
│   │   └── JsonTree.tsx     # 树形视图
│   └── src/utils/
│       ├── jsonUtils.ts     # JSON 工具函数
│       └── treeUtils.ts     # 树形操作函数
└── manifest.json
```

**关键方法**:
- `format_json`: 格式化 JSON
- `minify_json`: 压缩 JSON
- `escape_json`: 转义特殊字符
- `unescape_json`: 还原转义序列
```

**Step 3: 创建 CHANGELOG**

Create: `plugins/json-tools/CHANGELOG.md`

```markdown
# Changelog

## [1.0.0] - 2026-03-05

### Added
- 初始版本发布
- JSON 格式化功能
- JSON 压缩功能
- JSON 转义/去转义功能
- 树形视图可视化
- 节点选择和删除功能
- 全展开/全折叠功能
- 实时 JSON 语法验证
```

**Step 4: 最终提交**

```bash
git add README.md CLAUDE.md plugins/json-tools/CHANGELOG.md
git commit -m "docs: update project documentation for json-tools plugin"
```

---

## 完成检查清单

在标记任务完成之前,确认以下所有项目都已完成:

- [ ] 插件项目结构创建完成
- [ ] 后端 JSON 处理方法实现完成
- [ ] 前端项目结构创建完成
- [ ] 前端工具函数实现完成
- [ ] 工具栏组件实现完成
- [ ] 文本编辑器组件实现完成
- [ ] 树形视图组件实现完成
- [ ] 主应用组件实现完成
- [ ] CSS 样式定义完成
- [ ] 前端构建成功
- [ ] 插件动态库构建成功
- [ ] 插件打包成功
- [ ] 插件安装和测试通过
- [ ] 文档编写完成
- [ ] 代码质量检查通过
- [ ] 最终验证通过

---

## 相关文档

- [设计文档](./2026-03-05-json-tools-design.md)
- [插件开发指南](../../CLAUDE.md#插件开发规范)
- [密码管理器参考实现](../../plugins/password-manager/)

---

**实现计划版本**: 1.0.0
**创建日期**: 2026-03-05
**最后更新**: 2026-03-05
