# 文本比对插件设计文档

**创建日期**: 2025-03-11
**作者**: Claude AI
**状态**: 设计阶段

## 1. 概述

### 1.1 功能描述

文本比对插件是一个基于 Monaco Editor 的双栏文本比对工具,支持实时差异高亮、文件导入导出、差异导航等功能。

### 1.2 核心特性

- ✅ 左右双栏实时比对 (Monaco Diff Editor)
- ✅ 差异高亮显示 (添加/删除/修改)
- ✅ 差异导航 (上一个/下一个)
- ✅ 文件导入/导出 (支持多种文本格式)
- ✅ 比对选项 (忽略空白/忽略大小写)
- ✅ 差异统计 (添加/删除/修改行数)
- ✅ 导出 Unified Diff 格式
- ✅ 键盘快捷键支持 (F8/Shift+F8)

## 2. 架构设计

### 2.1 插件结构

```
text-diff/
├── src/
│   └── lib.rs              # Rust 后端 (文本预处理)
├── frontend/               # React + Vite 前端项目
│   ├── src/
│   │   ├── main.tsx        # 应用入口
│   │   ├── App.tsx         # 主应用组件
│   │   ├── DiffEditor.tsx  # Monaco Diff Editor 封装
│   │   ├── Toolbar.tsx     # 工具栏组件
│   │   └── types.ts        # TypeScript 类型定义
│   ├── package.json
│   ├── vite.config.ts
│   └── tsconfig.json
├── assets/                 # 构建产物 (复制自 frontend/dist)
│   ├── index.html
│   ├── main.js
│   └── styles.css
├── manifest.json          # 插件元数据
└── Cargo.toml
```

### 2.2 技术栈

**后端 (Rust)**:
- `worktools-plugin-api` - 插件 API
- `serde_json` - JSON 序列化
- `anyhow` - 错误处理

**前端 (React)**:
- `React 18` - UI 框架
- `Monaco Editor` - 代码编辑器 (VS Code 内核)
- `Vite` - 构建工具
- `TypeScript` - 类型安全

### 2.3 数据流向

```
用户操作
  → Toolbar 组件 (React)
  → 调用 Rust 后端 API
  → 返回处理结果
  → 更新 Diff Editor 视图
```

## 3. Rust 后端 API 设计

### 3.1 插件元数据

```rust
impl Plugin for TextDiff {
    fn id(&self) -> &str { "text-diff" }
    fn name(&self) -> &str { "文本比对" }
    fn description(&self) -> &str { "实时文本比对工具" }
    fn version(&self) -> &str { "1.0.0" }
    fn icon(&self) -> &str { "🔍" }
}
```

### 3.2 API 方法

#### 3.2.1 加载文本文件

```rust
"load_text_file"
参数: { file_path: string }
返回: { content: string, encoding: string }
错误: FILE_NOT_FOUND, FILE_TOO_LARGE
```

#### 3.2.2 保存文本文件

```rust
"save_text_file"
参数: { file_path: string, content: string }
返回: { success: boolean }
```

#### 3.2.3 预处理文本

```rust
"preprocess_text"
参数: {
  text: string,
  ignore_whitespace: boolean,
  ignore_case: boolean
}
返回: { original: string, processed: string }
```

#### 3.2.4 计算差异统计

```rust
"count_diff"
参数: { original: string, modified: string }
返回: {
  additions: number,
  deletions: number,
  modifications: number
}
```

#### 3.2.5 导出差异

```rust
"export_diff"
参数: {
  original: string,
  modified: string,
  filename: string
}
返回: { diff: string } // Unified Diff 格式
```

### 3.3 错误处理

```rust
// 文件不存在
{
  "error": "文件不存在",
  "code": "FILE_NOT_FOUND"
}

// 文件过大 (>10MB)
{
  "error": "文件过大 (最大 10MB)",
  "code": "FILE_TOO_LARGE"
}

// 不支持的文件类型
{
  "error": "不支持的文件类型",
  "code": "UNSUPPORTED_FILE_TYPE"
}
```

## 4. React 前端设计

### 4.1 组件结构

```typescript
App (主应用)
├── ErrorBoundary (错误边界)
│   └── MainApp
│       ├── Toolbar (工具栏)
│       │   ├── FileButtons (文件操作)
│       │   ├── NavButtons (导航按钮)
│       │   ├── Options (选项)
│       │   ├── ExportButtons (导出)
│       │   └── Stats (统计信息)
│       └── DiffEditor (比对视图)
```

### 4.2 核心组件

#### 4.2.1 DiffEditor.tsx

```typescript
interface DiffEditorProps {
  originalText: string;
  modifiedText: string;
  options: DiffOptions;
}

export function DiffEditor({ originalText, modifiedText, options }: DiffEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<monaco.editor.IStandaloneDiffEditor | null>(null);

  // 初始化 Monaco Diff Editor
  useEffect(() => {
    if (!containerRef.current) return;

    editorRef.current = monaco.editor.createDiffEditor(containerRef.current, {
      enableSplitViewResizing: true,
      renderSideBySide: true,
      ignoreTrimWhitespace: options.ignoreWhitespace,
      readOnly: false
    });

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
  }, [originalText, modifiedText]);

  return <div ref={containerRef} className="diff-editor-container" />;
}
```

#### 4.2.2 Toolbar.tsx

```typescript
export function Toolbar() {
  const [ignoreWhitespace, setIgnoreWhitespace] = useState(false);
  const [ignoreCase, setIgnoreCase] = useState(false);
  const [diffStats, setDiffStats] = useState({ additions: 0, deletions: 0, modifications: 0 });
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleFileOpen = useCallback(async (side: 'left' | 'right') => {
    setIsLoading(true);
    setError(null);

    try {
      const selected = await open({
        multiple: false,
        filters: FILE_FILTERS
      });

      if (!selected) return;

      const result = await window.pluginAPI.call('text-diff', 'load_text_file', {
        file_path: selected
      });

      // 更新对应侧的文本
      if (side === 'left') {
        setOriginalText(result.content);
      } else {
        setModifiedText(result.content);
      }
    } catch (err) {
      setError(`加载失败: ${err.message}`);
    } finally {
      setIsLoading(false);
    }
  }, []);

  return (
    <div className="toolbar">
      <button onClick={() => handleFileOpen('left')}>打开左侧文件</button>
      <button onClick={() => handleFileOpen('right')}>打开右侧文件</button>

      <div className="separator" />

      <button onClick={handlePreviousDiff}>↑ 上一个差异</button>
      <button onClick={handleNextDiff}>↓ 下一个差异</button>

      <div className="separator" />

      <label>
        <input type="checkbox" checked={ignoreWhitespace} onChange={(e) => setIgnoreWhitespace(e.target.checked)} />
        忽略空白
      </label>

      <label>
        <input type="checkbox" checked={ignoreCase} onChange={(e) => setIgnoreCase(e.target.checked)} />
        忽略大小写
      </label>

      <div className="separator" />

      <button onClick={handleExport}>导出差异</button>

      <div className="stats">
        添加: {diffStats.additions} | 删除: {diffStats.deletions} | 修改: {diffStats.modifications}
      </div>
    </div>
  );
}
```

### 4.3 样式设计

```css
/* 工具栏样式 */
.toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: #1e1e1e;
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
}

.toolbar button:hover {
  background: #005a9e;
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
}

.toolbar .stats {
  margin-left: auto;
  color: #cccccc;
  font-size: 13px;
}

/* Diff Editor 容器 */
.diff-editor-container {
  width: 100%;
  height: calc(100vh - 60px);
  border: 1px solid #3e3e42;
}

/* 差异高亮颜色 */
.monaco-editor .diagonal-fill {
  background-color: #ffeba0;
}

.monaco-editor .line-add {
  background-color: #c6f6d5;
}

.monaco-editor .line-delete {
  background-color: #fed7d7;
}
```

## 5. 功能流程

### 5.1 文件导入流程

```
用户点击"打开左侧文件"按钮
  → 调用 Tauri open() API 选择文件
  → 调用 Rust: load_text_file(file_path)
  → 验证文件存在性和大小
  → 读取文件内容
  → 返回 { content, encoding }
  → 更新 originalText 状态
  → DiffEditor 自动更新视图
```

### 5.2 差异比对流程

```
用户输入文本或打开文件
  → Monaco Diff Editor 自动计算差异
  → 高亮显示差异 (添加/删除/修改)
  → 调用 Rust: count_diff(original, modified)
  → 返回统计信息
  → 更新工具栏显示
```

### 5.3 差异导航流程

```
用户点击"下一个差异"按钮
  → 调用 editorRef.current.goToDiff('next')
  → Monaco 自动跳转到下一个差异位置
  → 高亮当前行
  → 更新差异位置指示器
```

### 5.4 选项切换流程

```
用户勾选"忽略空白"
  → ignoreWhitespace 状态变化
  → useEffect 触发
  → 调用 Rust: preprocess_text(text, { ignore_whitespace: true })
  → 返回处理后的文本
  → 更新 Diff Editor 视图
  → 重新计算差异
```

### 5.5 导出差异流程

```
用户点击"导出差异"按钮
  → 调用 Rust: export_diff(original, modified, filename)
  → 生成 Unified Diff 格式
  → 调用 Tauri save() API 选择保存位置
  → 调用 Rust: save_text_file(file_path, diff_content)
  → 保存文件
```

## 6. 边界情况处理

### 6.1 空文本比对

- 原始文本和修改文本都为空 → 显示"无差异"
- 只有一侧为空 → 正常高亮另一侧内容

### 6.2 大文件处理

- 文件大小限制: 10MB
- 超过限制时显示错误提示
- 建议用户使用专门的 diff 工具

### 6.3 特殊字符处理

- Unicode 字符 (emoji) → Monaco 原生支持
- 不同换行符 (\r\n vs \n) → 自动规范化
- Tab 字符 → 保留原始格式

### 6.4 二进制文件检测

```rust
// 检测文件是否包含空字节
fn is_binary_file(content: &[u8]) -> bool {
    content.contains(&0)
}
```

### 6.5 性能优化

- 防抖输入: 300ms 延迟更新 Diff Editor
- 虚拟滚动: Monaco 自动处理大文件
- 内存清理: 组件卸载时调用 `editor.dispose()`

## 7. 支持的文件格式

```typescript
const FILE_FILTERS = [
  {
    name: 'All Text Files',
    extensions: ['txt', 'md', 'js', 'ts', 'jsx', 'tsx', 'json', 'html', 'css', 'scss', 'yaml', 'yml', 'xml', 'diff', 'patch']
  },
  { name: 'Plain Text', extensions: ['txt'] },
  { name: 'Markdown', extensions: ['md'] },
  { name: 'JavaScript/TypeScript', extensions: ['js', 'ts', 'jsx', 'tsx'] },
  { name: 'JSON', extensions: ['json'] },
  { name: 'HTML/CSS', extensions: ['html', 'css', 'scss'] }
];
```

## 8. 键盘快捷键

| 快捷键 | 功能 |
|--------|------|
| F8 | 下一个差异 |
| Shift+F8 | 上一个差异 |
| Ctrl+O (Cmd+O) | 打开左侧文件 |
| Ctrl+Shift+O (Cmd+Shift+O) | 打开右侧文件 |
| Ctrl+S (Cmd+S) | 导出差异 |

## 9. 依赖清单

### 9.1 Rust 依赖

```toml
[dependencies]
worktools-plugin-api = { path = "../../shared/plugin-api" }
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
similar = "2.4"  # diff 算法库
```

### 9.2 前端依赖

```json
{
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "monaco-editor": "^0.45.0"
  },
  "devDependencies": {
    "@types/react": "^18.2.0",
    "@types/react-dom": "^18.2.0",
    "@vitejs/plugin-react": "^4.2.0",
    "typescript": "^5.3.0",
    "vite": "^5.0.0"
  }
}
```

## 10. 实施计划

### 10.1 阶段一: 基础架构 (1-2 天)

- [ ] 创建插件项目结构
- [ ] 配置 Vite + React 开发环境
- [ ] 实现 Rust 后端基础 API
- [ ] 创建 manifest.json

### 10.2 阶段二: 核心功能 (2-3 天)

- [ ] 实现 Monaco Diff Editor 封装
- [ ] 实现文件导入/导出功能
- [ ] 实现差异统计
- [ ] 实现差异导航

### 10.3 阶段三: 增强功能 (1-2 天)

- [ ] 实现比对选项 (忽略空白/大小写)
- [ ] 实现键盘快捷键
- [ ] 实现错误处理
- [ ] 添加加载状态和提示

### 10.4 阶段四: 测试和打包 (1 天)

- [ ] 测试各种边界情况
- [ ] 性能测试
- [ ] 打包为 .wtplugin.zip
- [ ] 编写使用文档

## 11. 未来扩展

### 11.1 可能的增强功能

- 支持多文件批量比对
- 并排 vs 统一 diff 视图切换
- 差异历史记录保存
- 支持三方合并 (3-way merge)
- 支持 Git 仓库直接比对

### 11.2 性能优化

- Web Worker 后台处理大文件
- 增量 diff 算法
- 虚拟滚动优化

## 12. 参考资料

- [Monaco Editor Documentation](https://microsoft.github.io/monaco-editor/)
- [Tauri Documentation](https://tauri.app/)
- [Work Tools Platform 插件开发指南](../../README.md)
- [菜鸟工具在线文本比对](https://www.jyshare.com/front-end/8006/)
