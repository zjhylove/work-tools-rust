# 文本比对工具架构重构设计文档

**日期**: 2026-03-17
**版本**: 1.0
**作者**: Claude Sonnet + zj
**状态**: 设计阶段

---

## 目录

1. [问题分析](#问题分析)
2. [架构设计](#架构设计)
3. [组件设计](#组件设计)
4. [数据流设计](#数据流设计)
5. [核心功能实现](#核心功能实现)
6. [性能优化](#性能优化)
7. [测试策略](#测试策略)
8. [实施计划](#实施计划)

---

## 问题分析

### 现存问题总结

#### 1. 代码质量和组织问题 (4个)
- **重复文件冲突**: `frontend/DiffEditor.tsx` (旧,引用monaco) 和 `frontend/src/DiffEditor.tsx` (新,使用diff) 同时存在
- **重复测试代码**: `lib.rs` 中有两个相同的 `test_export_diff` 函数 (lines 349-360 和 363-378)
- **未清理资源**: `assets/codicon.ttf` (119KB) 和 Monaco Editor 相关样式文件未删除
- **功能被注释**: 差异统计功能被完全注释掉,导致统计永远显示为 0

#### 2. 功能缺失 (5个)
- **差异导航未实现**: `goToDiff` 只有 TODO 注释,点击按钮无效果
- **文件选择体验极差**: 使用 `prompt()` 手动输入路径,无法通过文件浏览器选择
- **缺少同步滚动**: 左右面板滚动不同步,难以对应查看
- **只有并排视图**: 没有行内对比模式,小窗口下体验差
- **大文件处理未验证**: 10MB 限制但无进度提示、无分块加载

#### 3. 实际功能失效 (5个) - 最严重
- **文件选择按钮无响应**: `prompt()` 在 iframe 环境被阻止
- **无法编辑/粘贴**: 使用 `<span>` 只读渲染,无法修改内容
- **忽略空白无效**: 选项存在但不生效(需验证具体原因)
- **导出按钮无响应**: 调用后端失败但没有错误提示
- **面板名称固定**: "原始文件"/"修改后的文件"不显示实际文件名

#### 4. 用户体验问题 (5个)
- **错误提示不友好**: 3秒自动消失,无详细信息和解决建议
- **缺少加载状态**: 加载大文件时无进度指示
- **快捷键冲突**: `Ctrl+O` 与浏览器命令冲突
- **视觉反馈不足**: 无悬停增强、无对应线连接、无当前位置指示
- **响应式设计缺失**: 小窗口和移动设备不可用

---

## 架构设计

### 核心理念

从 **"只读 diff 显示器"** 转变为 **"可编辑差异对比工作台"**

### 架构分层

```
┌─────────────────────────────────────────────┐
│           展示层 (Presentation)              │
│  React + TypeScript + Tailwind CSS          │
│  单向数据流 (useState + useReducer)          │
└─────────────────────────────────────────────┘
                     ↕
┌─────────────────────────────────────────────┐
│         业务逻辑层 (Business Logic)          │
│  前端: diff 库 (实时差异计算)                 │
│  后端: similar 库 (大文件处理、导出)          │
└─────────────────────────────────────────────┘
                     ↕
┌─────────────────────────────────────────────┐
│            数据层 (Data Layer)               │
│  内存: React State                           │
│  持久化: ~/.worktools/history/plugins/      │
│  缓存: Diff 结果缓存                         │
└─────────────────────────────────────────────┘
```

### 前后端职责分工

| 功能 | 前端职责 | 后端职责 |
|------|---------|---------|
| 文件读取 | 调用 Tauri API | 文件系统 I/O、编码检测 |
| 差异计算 | 实时计算 (小文件<1MB) | 批量计算 (大文件) |
| 差异渲染 | 高亮显示、交互反馈 | 无 |
| 文件保存 | 调用 Tauri API | 文件系统 I/O |
| 格式导出 | 触发导出 | Unified Diff 生成 |
| 配置管理 | 运行时配置 | 持久化存储 |

### 关键技术决策

#### 1. 可编辑实现方案
- **方案选择**: 使用 `textarea` + 覆盖层实现
- **原因**: 相比 Monaco Editor (14.5MB) 轻量得多,且满足基本需求
- **权衡**: 牺牲部分高级编辑功能(如多光标),换取极简的体积和加载速度

#### 2. 文件对话框
- **方案**: 通过 Tauri IPC 调用系统原生对话框
- **API**: `invoke('dialog_open', { options })`
- **原因**: 用户体验最佳,支持路径验证和过滤

#### 3. 性能优化策略
- **阈值判定**:
  - 文件 < 1MB: 前端同步计算
  - 文件 1-10MB: 前端异步计算 (requestIdleCallback)
  - 文件 > 10MB: 拒绝加载或提示风险
- **虚拟滚动**: 只渲染可见区域 ± 10 行
- **防抖**: 用户输入停止 300ms 后才重新计算 diff

---

## 组件设计

### 组件树结构

```
App (根组件)
├── ErrorBoundary (错误边界)
├── AppHeader (顶部工具栏)
│   ├── FilePickerGroup (文件选择组)
│   │   ├── FilePickerButton (左文件按钮)
│   │   └── FilePickerButton (右文件按钮)
│   ├── ViewModeToggle (视图模式: 并排/行内)
│   ├── DiffOptions (差异选项)
│   │   ├── Checkbox (忽略空白)
│   │   ├── Checkbox (忽略大小写)
│   │   └── Toggle (同步滚动)
│   ├── ExportButton (导出按钮)
│   └── DiffStats (差异统计: ➕X ➖Y ✏️Z)
├── DiffEditor (核心差异编辑器)
│   ├── EditorPane (左编辑器)
│   │   ├── FileHeader (文件名/路径/图标)
│   │   ├── LineNumberColumn (行号列)
│   │   │   └── LineNumber (单行号)
│   │   ├── HighlightLayer (差异高亮层)
│   │   │   └── HighlightedLine (高亮行)
│   │   └── TextAreaLayer (编辑层)
│   │       └── ResizableTextArea (可调整大小)
│   ├── Divider (分割线/拖动条)
│   └── EditorPane (右编辑器)
│       └── ... (同上)
├── NavigationBar (差异导航栏)
│   ├── JumpToPrevious (↑ 上一个)
│   ├── DiffCounter (1/15)
│   └── JumpToNext (↓ 下一个)
└── StatusBar (底部状态栏)
    ├── FileNames (file1.txt vs file2.txt)
    ├── CursorPosition (行 12, 列 5)
    ├── EncodingInfo (UTF-8)
    └── LoadingIndicator (计算中...)
```

### 核心组件详解

#### 1. DiffEditor 组件

**职责**:
- 协调两个 EditorPane 的状态同步
- 管理差异计算和缓存
- 实现同步滚动逻辑
- 处理差异导航

**状态**:
```typescript
interface DiffEditorState {
  originalText: string;
  modifiedText: string;
  diffLines: DiffLine[];
  currentDiffIndex: number;
  viewMode: 'side-by-side' | 'inline';
  isCalculating: boolean;
  scrollSync: boolean;
  options: {
    ignoreWhitespace: boolean;
    ignoreCase: boolean;
  };
}

interface DiffLine {
  originalLine: number | null;  // 原文件行号(插入时为null)
  modifiedLine: number | null;  // 修改文件行号(删除时为null)
  type: 'delete' | 'insert' | 'equal';
  originalContent: string;
  modifiedContent: string;
}
```

**关键方法**:
```typescript
// 1. 同步滚动算法
const syncScroll = (
  sourcePane: 'left' | 'right',
  scrollTop: number
) => {
  const leftHeight = leftPaneRef.current?.scrollHeight || 1;
  const rightHeight = rightPaneRef.current?.scrollHeight || 1;
  const ratio = rightHeight / leftHeight;

  if (sourcePane === 'left' && rightPaneRef.current) {
    rightPaneRef.current.scrollTop = scrollTop * ratio;
  } else if (sourcePane === 'right' && leftPaneRef.current) {
    leftPaneRef.current.scrollTop = scrollTop / ratio;
  }
};

// 2. 实时差异计算 (防抖)
const recalculateDiff = useMemo(
  () => debounce(() => {
    setIsCalculating(true);

    const changes = diffLines(
      preprocessText(originalText, options),
      preprocessText(modifiedText, options)
    );

    setDiffLines(convertToDiffLines(changes));
    setIsCalculating(false);
  }, 300),
  [originalText, modifiedText, options]
);

// 3. 差异导航
const navigateDiff = (direction: 'next' | 'previous') => {
  const diffIndices = diffLines
    .map((line, idx) => line.type !== 'equal' ? idx : -1)
    .filter(idx => idx !== -1);

  if (diffIndices.length === 0) return;

  const currentIdx = diffIndices.indexOf(currentDiffIndex);
  const nextIdx = direction === 'next'
    ? Math.min(currentIdx + 1, diffIndices.length - 1)
    : Math.max(currentIdx - 1, 0);

  setCurrentDiffIndex(diffIndices[nextIdx]);
  scrollToLine(diffIndices[nextIdx]);
  highlightLine(diffIndices[nextIdx]);
};
```

#### 2. EditorPane 组件

**职责**:
- 渲染可编辑的文本区域
- 显示行号
- 高亮差异行
- 处理用户输入

**双层架构**:
```
┌────────────────────────────────────┐
│  LineNumberColumn  │  ContentArea  │
├────────────────────────────────────┤
│  1               │  ┌─────────────┴──┐
│  2               │  │ HighlightLayer  │ (透明,高亮背景)
│  3               │  ├─────────────────┤
│  4               │  │ TextAreaLayer   │ (可编辑)
│  5               │  └─────────────────┘
└────────────────────────────────────┘
```

**实现要点**:
```typescript
// 样式层叠
.line-content {
  position: relative;
}

.highlight-layer {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  pointer-events: none;  /* 让点击穿透到 textarea */
  z-index: 1;
}

.textarea-layer {
  position: relative;
  width: 100%;
  height: 100%;
  border: none;
  background: transparent;
  color: transparent;  /* 文字透明,显示高亮层 */
  caret-color: #000;   /* 但光标可见 */
  resize: none;
  font-family: monospace;
  z-index: 2;
}
```

#### 3. FilePickerGroup 组件

**职责**:
- 提供文件选择按钮
- 调用 Tauri 文件对话框
- 显示当前选中的文件名
- 支持拖放文件

**Tauri IPC 调用**:
```typescript
const handleFilePick = async (side: 'left' | 'right') => {
  try {
    // 调用 Tauri 对话框
    const selected = await invoke<string>('dialog_open', {
      options: {
        title: side === 'left' ? '选择原始文件' : '选择修改后的文件',
        filters: [
          { name: '文本文件', extensions: ['txt', 'md', 'js', 'ts', 'json'] },
          { name: '所有文件', extensions: ['*'] }
        ],
        multiple: false
      }
    });

    if (!selected) return;

    // 加载文件内容
    const content = await invoke<string>('read_file', { path: selected });

    // 更新对应面板的内容
    if (side === 'left') {
      setOriginalText(content);
      setOriginalFileName(extractFileName(selected));
    } else {
      setModifiedText(content);
      setModifiedFileName(extractFileName(selected));
    }
  } catch (err) {
    showError(`无法打开文件: ${err.message}`);
  }
};

// 拖放支持
const handleDrop = (e: DragEvent, side: 'left' | 'right') => {
  e.preventDefault();
  const file = e.dataTransfer.files[0];
  if (file) {
    const reader = new FileReader();
    reader.onload = (event) => {
      const content = event.target?.result as string;
      if (side === 'left') {
        setOriginalText(content);
        setOriginalFileName(file.name);
      } else {
        setModifiedText(content);
        setModifiedFileName(file.name);
      }
    };
    reader.readAsText(file);
  }
};
```

#### 4. DiffOptions 组件

**职责**:
- 提供差异计算选项
- 实时更新 DiffEditor
- 保持选项状态同步

**选项列表**:
- 忽略空白字符 (ignoreWhitespace)
- 忽略大小写 (ignoreCase)
- 同步滚动 (scrollSync)
- 视图模式 (并排/行内)

---

## 数据流设计

### 状态管理架构

采用 **单向数据流** + **本地状态** 模式:

```
User Action (用户操作)
      ↓
Event Handler (事件处理器)
      ↓
State Update (状态更新)
      ↓
Re-render (重新渲染)
      ↓
View Update (视图更新)
```

### 数据流示例: 文件加载流程

```
用户点击"打开左文件"
      ↓
FilePickerButton.onClick
      ↓
invoke('dialog_open', options)
      ↓
[Tauri Backend] 系统文件对话框
      ↓
返回文件路径
      ↓
invoke('read_file', path)
      ↓
[Tauri Backend] 读取文件内容
      ↓
返回文件内容
      ↓
setOriginalText(content)
setOriginalFileName(fileName)
      ↓
useEffect 触发 recalculateDiff()
      ↓
diffLines(originalText, modifiedText)
      ↓
setDiffLines(changes)
      ↓
DiffEditor 重新渲染,显示差异高亮
```

### 数据流示例: 实时编辑流程

```
用户在左面板输入文本
      ↓
TextAreaLayer.onChange
      ↓
debounce(300ms) // 防抖
      ↓
setOriginalText(newText)
      ↓
useEffect 监听 originalText 变化
      ↓
recalculateDiff()
      ↓
diffLines(newText, modifiedText)
      ↓
setDiffLines(newChanges)
      ↓
重新渲染高亮层
```

### 状态持久化

**需要持久化的状态**:
- 最近打开的文件路径 (最近10个)
- 差异选项设置 (ignoreWhitespace, ignoreCase)
- 视图模式偏好 (side-by-side / inline)
- 面板分割比例

**持久化时机**:
- 状态变化时立即保存 (debounce 1s)
- 应用关闭前保存
- 使用 Tauri API: `invoke('save_plugin_config', { key, value })`

---

## 核心功能实现

### 1. 差异计算引擎

**前端计算 (文件 < 1MB)**:
```typescript
import { diffLines, diffWords } from 'diff';

const calculateDiff = (
  original: string,
  modified: string,
  options: DiffOptions
): DiffLine[] => {
  // 预处理
  const processedOriginal = preprocess(original, options);
  const processedModified = preprocess(modified, options);

  // 计算差异
  const changes = diffLines(processedOriginal, processedModified);

  // 转换为 DiffLine[]
  const diffLines: DiffLine[] = [];
  let originalLineNum = 1;
  let modifiedLineNum = 1;

  changes.forEach((part) => {
    const lines = part.value.split('\n').filter(l => l !== '');

    if (part.removed) {
      // 删除的行
      lines.forEach((line) => {
        diffLines.push({
          originalLine: originalLineNum++,
          modifiedLine: null,
          type: 'delete',
          originalContent: line,
          modifiedContent: ''
        });
      });
    } else if (part.added) {
      // 新增的行
      lines.forEach((line) => {
        diffLines.push({
          originalLine: null,
          modifiedLine: modifiedLineNum++,
          type: 'insert',
          originalContent: '',
          modifiedContent: line
        });
      });
    } else {
      // 相同的行
      lines.forEach((line) => {
        diffLines.push({
          originalLine: originalLineNum++,
          modifiedLine: modifiedLineNum++,
          type: 'equal',
          originalContent: line,
          modifiedContent: line
        });
      });
    }
  });

  return diffLines;
};
```

**后端计算 (文件 >= 1MB)**:
```rust
use similar::{Algorithm, ChangeTag, TextDiff};

#[tauri::command]
pub async fn calculate_diff_large(
    original: String,
    modified: String,
    options: ProcessOptions
) -> Result<Vec<DiffLine>, String> {
    // 预处理
    let original = preprocess_text(&original, &options);
    let modified = preprocess_text(&modified, &options);

    // 使用 Patience 算法 (对代码更友好)
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Patience)
        .diff_lines(&original, &modified);

    // 转换为 DiffLine 数组
    let mut result = Vec::new();
    let mut orig_line = 1;
    let mut mod_line = 1;

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => {
                result.push(DiffLine {
                    original_line: Some(orig_line),
                    modified_line: None,
                    type_: "delete".to_string(),
                    original_content: change.value().to_string(),
                    modified_content: String::new(),
                });
                orig_line += 1;
            }
            ChangeTag::Insert => {
                result.push(DiffLine {
                    original_line: None,
                    modified_line: Some(mod_line),
                    type_: "insert".to_string(),
                    original_content: String::new(),
                    modified_content: change.value().to_string(),
                });
                mod_line += 1;
            }
            ChangeTag::Equal => {
                result.push(DiffLine {
                    original_line: Some(orig_line),
                    modified_line: Some(mod_line),
                    type_: "equal".to_string(),
                    original_content: change.value().to_string(),
                    modified_content: change.value().to_string(),
                });
                orig_line += 1;
                mod_line += 1;
            }
        }
    }

    Ok(result)
}
```

### 2. 文件加载和保存

**Tauri Commands 扩展**:
```rust
#[tauri::command]
pub async fn open_file_dialog() -> Result<String, String> {
    // 使用 Tauri 的 dialog API
    // 这里需要实际实现文件对话框逻辑
    Err("未实现".to_string())
}

#[tauri::command]
pub async fn read_file_content(path: String) -> Result<String, String> {
    use std::fs;
    use std::path::Path;

    if !Path::new(&path).exists() {
        return Err("文件不存在".to_string());
    }

    let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;

    if metadata.len() > 10 * 1024 * 1024 {
        return Err("文件过大 (最大 10MB)".to_string());
    }

    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_file_content(path: String, content: String) -> Result<(), String> {
    use std::fs;

    fs::write(&path, content).map_err(|e| e.to_string())
}
```

### 3. 导出功能

**Unified Diff 导出**:
```typescript
const exportUnifiedDiff = async () => {
  try {
    // 调用后端生成 Unified Diff
    const result = await invoke('export_unified_diff', {
      original: originalText,
      modified: modifiedText,
      filename: 'changes.diff',
      contextLines: 3  // 显示前后3行上下文
    });

    // 创建下载链接
    const blob = new Blob([result], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `diff-${Date.now()}.diff`;
    a.click();
    URL.revokeObjectURL(url);

    showSuccess('导出成功');
  } catch (err) {
    showError(`导出失败: ${err.message}`);
  }
};
```

**后端实现** (已有,在 lib.rs 中):
```rust
fn export_unified_diff(original: &str, modified: &str, filename: &str) -> String {
    let diff = SimilarTextDiff::configure()
        .algorithm(Algorithm::Patience)
        .diff_lines(original, modified);

    // ... (实现已在 lib.rs:113-182)
}
```

### 4. 差异导航

**导航逻辑**:
```typescript
const navigateDiff = (direction: 'next' | 'previous') => {
  // 找出所有差异行的索引
  const diffIndices = diffLines
    .map((line, idx) => line.type !== 'equal' ? idx : -1)
    .filter(idx => idx !== -1);

  if (diffIndices.length === 0) {
    showInfo('没有差异');
    return;
  }

  // 计算当前焦点位置
  const currentIdx = diffIndices.indexOf(currentDiffIndex);

  // 计算下一个位置
  let nextIdx: number;
  if (direction === 'next') {
    nextIdx = currentIdx < diffIndices.length - 1
      ? currentIdx + 1
      : 0;  // 循环到第一个
  } else {
    nextIdx = currentIdx > 0
      ? currentIdx - 1
      : diffIndices.length - 1;  // 循环到最后一个
  }

  const targetLine = diffIndices[nextIdx];

  // 滚动到目标行
  const lineElement = document.querySelector(
    `[data-line-index="${targetLine}"]`
  );
  lineElement?.scrollIntoView({ behavior: 'smooth', block: 'center' });

  // 高亮当前差异
  setCurrentDiffIndex(targetLine);

  // 更新导航计数器
  setDiffCounter(`${nextIdx + 1} / ${diffIndices.length}`);
};
```

### 5. 视图模式切换

**并排模式 (Side-by-Side)**:
```
┌─────────────┬─────────────┐
│  Original   │  Modified   │
│  File A.txt │  File B.txt │
├─────────────┼─────────────┤
│ line 1      │ line 1      │
│ line 2      │ line 2      │
│ - line 3    │ + line 3'   │  ← 差异
│ line 4      │ line 4      │
└─────────────┴─────────────┘
```

**行内模式 (Inline)**:
```
┌───────────────────────────┐
│  Comparison: A.txt → B.txt│
├───────────────────────────┤
│ line 1                    │
│ line 2                    │
│ - line 3   (removed)      │  ← 红色
│ + line 3'  (added)        │  ← 绿色
│ line 4                    │
└───────────────────────────┘
```

**实现**:
```typescript
const InlineView = ({ diffLines }: { diffLines: DiffLine[] }) => {
  return (
    <div className="inline-view">
      {diffLines.map((line, idx) => (
        <div
          key={idx}
          className={`diff-line-${line.type}`}
          data-line-index={idx}
        >
          <span className="line-marker">
            {line.type === 'delete' ? '-' : line.type === 'insert' ? '+' : ' '}
          </span>
          <span className="line-number">
            {line.originalLine ?? line.modifiedLine ?? ''}
          </span>
          <span className="line-content">
            {line.originalContent || line.modifiedContent}
          </span>
        </div>
      ))}
    </div>
  );
};
```

---

## 性能优化

### 1. 虚拟滚动

**问题**: 大文件(10000+ 行)渲染所有 DOM 节点导致卡顿

**解决方案**: 只渲染可见区域的行

```typescript
const VirtualScroll = ({
  lines,
  rowHeight = 22,
  visibleRows = 30
}: {
  lines: DiffLine[];
  rowHeight?: number;
  visibleRows?: number;
}) => {
  const [scrollTop, setScrollTop] = useState(0);

  // 计算可见范围
  const startIndex = Math.floor(scrollTop / rowHeight);
  const endIndex = Math.min(startIndex + visibleRows, lines.length);

  // 计算偏移量
  const offsetY = startIndex * rowHeight;
  const totalHeight = lines.length * rowHeight;

  return (
    <div
      className="virtual-scroll-container"
      onScroll={(e) => setScrollTop(e.currentTarget.scrollTop)}
      style={{ height: `${visibleRows * rowHeight}px` }}
    >
      <div
        className="virtual-scroll-spacer"
        style={{ height: `${totalHeight}px`, position: 'relative' }}
      >
        <div
          className="virtual-scroll-content"
          style={{ transform: `translateY(${offsetY}px)` }}
        >
          {lines.slice(startIndex, endIndex).map((line, idx) => (
            <DiffLineItem
              key={startIndex + idx}
              line={line}
              index={startIndex + idx}
            />
          ))}
        </div>
      </div>
    </div>
  );
};
```

### 2. Web Worker 异步计算

**问题**: 大文件 diff 计算阻塞 UI 线程

**解决方案**: 使用 Web Worker 在后台计算

```typescript
// diff-worker.ts
import { diffLines } from 'diff';

self.onmessage = (e: MessageEvent) => {
  const { original, modified, options } = e.data;

  const result = diffLines(original, modified);

  self.postMessage({ result });
};

// 组件中使用
const useDiffWorker = () => {
  const [isCalculating, setIsCalculating] = useState(false);

  const calculateDiff = (
    original: string,
    modified: string,
    options: DiffOptions
  ): Promise<DiffLine[]> => {
    return new Promise((resolve) => {
      setIsCalculating(true);

      const worker = new Worker(new URL('./diff-worker.ts', import.meta.url));

      worker.postMessage({ original, modified, options });

      worker.onmessage = (e) => {
        resolve(e.data.result);
        setIsCalculating(false);
        worker.terminate();
      };
    });
  };

  return { calculateDiff, isCalculating };
};
```

### 3. 防抖和节流

**防抖 (Debounce)**: 用户输入停止后 300ms 才计算 diff
```typescript
import { debounce } from 'lodash-es';  // 或自己实现

const recalculateDiff = useMemo(
  () => debounce((original: string, modified: string) => {
    // 计算逻辑
  }, 300),
  []
);
```

**节流 (Throttle)**: 滚动事件每 100ms 最多触发一次
```typescript
import { throttle } from 'lodash-es';

const handleScroll = useMemo(
  () => throttle((scrollTop: number) => {
    // 同步滚动逻辑
  }, 100),
  []
);
```

### 4. React 优化

**React.memo**: 避免不必要的重渲染
```typescript
const DiffLineItem = React.memo(({ line, index }: {
  line: DiffLine;
  index: number;
}) => {
  return (
    <div className={`diff-line diff-line-${line.type}`}>
      {/* ... */}
    </div>
  );
});
```

**useMemo**: 缓存计算结果
```typescript
const diffStats = useMemo(() => {
  const additions = diffLines.filter(l => l.type === 'insert').length;
  const deletions = diffLines.filter(l => l.type === 'delete').length;
  const modifications = Math.min(additions, deletions);

  return { additions, deletions, modifications };
}, [diffLines]);
```

**useCallback**: 稳定的函数引用
```typescript
const handleScroll = useCallback((scrollTop: number) => {
  // 滚动逻辑
}, []);
```

---

## 测试策略

### 单元测试

**前端组件测试** (使用 Vitest + React Testing Library):
```typescript
// DiffEditor.test.tsx
import { render, screen, fireEvent } from '@testing-library/react';
import { DiffEditor } from './DiffEditor';

describe('DiffEditor', () => {
  it('应该正确计算差异', () => {
    render(
      <DiffEditor
        originalText="Line 1\nLine 2"
        modifiedText="Line 1\nModified"
        options={{ ignoreWhitespace: false, ignoreCase: false }}
      />
    );

    expect(screen.getByText(/Modified/)).toBeInTheDocument();
  });

  it('应该支持文本编辑', () => {
    const { container } = render(
      <DiffEditor
        originalText="Hello"
        modifiedText="World"
        options={{ ignoreWhitespace: false, ignoreCase: false }}
      />
    );

    const textarea = container.querySelector('textarea');
    fireEvent.change(textarea!, { target: { value: 'New Text' } });

    // 验证状态更新
  });
});
```

**后端单元测试**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_diff() {
        let original = "Hello\nWorld\nTest";
        let modified = "Hello\nRust\nTest";

        let stats = count_diff_lines(original, modified);

        assert_eq!(stats.additions, 1);
        assert_eq!(stats.deletions, 1);
        assert_eq!(stats.modifications, 0);
    }

    #[test]
    fn test_preprocess_text() {
        let text = "Hello  World\nTest   Line";
        let options = ProcessOptions {
            ignore_whitespace: true,
            ignore_case: false,
        };

        let processed = preprocess_text_impl(text, &options);

        assert_eq!(processed, "Hello World\nTest Line");
    }
}
```

### 集成测试

**文件加载流程**:
```typescript
describe('文件加载集成测试', () => {
  it('应该成功加载文件并显示差异', async () => {
    // Mock Tauri API
    global.invoke = mockfn()
      .with('dialog_open')
      .resolves('/tmp/test.txt')
      .with('read_file')
      .resolves('Line 1\nLine 2');

    render(<App />);

    fireEvent.click(screen.getByText('打开左文件'));

    await waitFor(() => {
      expect(screen.getByText('Line 1')).toBeInTheDocument();
    });
  });
});
```

### 性能测试

**大文件处理**:
```typescript
describe('性能测试', () => {
  it('应该在 1 秒内计算 10000 行的差异', async () => {
    const largeText = Array(10000).fill('Line').join('\n');

    const start = performance.now();
    await calculateDiff(largeText, largeText + '\nNew Line', {});
    const duration = performance.now() - start;

    expect(duration).toBeLessThan(1000);
  });
});
```

---

## 实施计划

### Phase 1: 清理和准备 (1-2天)

**任务**:
- [ ] 删除重复的 `frontend/DiffEditor.tsx` 文件
- [ ] 删除重复的测试代码
- [ ] 清理 Monaco Editor 资源文件 (codicon.ttf)
- [ ] 重构后的 CSS 清理
- [ ] 更新依赖项 (确保 diff@5.2.0 正确安装)

**验收标准**:
- 不再有重复文件
- node_modules 依赖干净
- 可以成功构建

### Phase 2: 核心架构重构 (3-4天)

**任务**:
- [ ] 创建新的组件结构
- [ ] 实现 `EditorPane` 组件 (可编辑)
- [ ] 实现 `DiffEditor` 核心逻辑
- [ ] 实现文件选择功能 (Tauri IPC)
- [ ] 修复差异计算和显示

**验收标准**:
- 可以打开文件并显示内容
- 可以直接在编辑器中编辑/粘贴文本
- 差异高亮正确显示
- 忽略空白选项生效

### Phase 3: 高级功能实现 (2-3天)

**任务**:
- [ ] 实现同步滚动
- [ ] 实现差异导航 (F8/Shift+F8)
- [ ] 修复导出功能
- [ ] 添加差异统计 (实时计算)
- [ ] 实现行内视图模式

**验收标准**:
- 左右面板同步滚动
- 点击导航按钮可以跳转到差异位置
- 导出按钮可以下载 .diff 文件
- 统计信息正确显示

### Phase 4: 性能优化和体验改进 (2-3天)

**任务**:
- [ ] 实现虚拟滚动 (超过 1000 行)
- [ ] 添加加载状态指示
- [ ] 优化大文件处理
- [ ] 改进错误提示
- [ ] 添加键盘快捷键帮助

**验收标准**:
- 10000 行文件滚动流畅
- 加载文件有进度提示
- 错误信息友好且有用
- 快捷键不冲突

### Phase 5: 测试和文档 (1-2天)

**任务**:
- [ ] 编写单元测试 (覆盖率 > 70%)
- [ ] 编写集成测试
- [ ] 更新 README.md
- [ ] 添加用户指南
- [ ] 性能测试和优化

**验收标准**:
- 所有测试通过
- 文档完整
- 性能满足要求

---

## 风险和依赖

### 风险

1. **iframe 环境限制**: Tauri API 调用可能受限
   - **缓解**: 提前测试 Tauri IPC 在 iframe 中的可用性

2. **性能瓶颈**: 大文件 diff 计算可能慢
   - **缓解**: 实现 Web Worker 和虚拟滚动

3. **兼容性问题**: 不同操作系统的文件对话框 API
   - **缓解**: 使用 Tauri 的跨平台 API

### 依赖

- Tauri 2.x IPC API
- diff@5.2.0 库
- similar@2.4 Rust 库
- React 19.x
- TypeScript 5.x

---

## 成功指标

### 功能完整性
- ✅ 所有现有功能正常工作
- ✅ 新增可编辑功能
- ✅ 新增文件对话框
- ✅ 新增差异导航

### 性能指标
- ✅ 10000 行文件加载 < 2 秒
- ✅ 差异计算 < 1 秒
- ✅ 滚动 FPS > 60

### 用户体验
- ✅ 错误信息清晰友好
- ✅ 加载状态可见
- ✅ 快捷键合理且不冲突
- ✅ 响应式设计支持小窗口

### 代码质量
- ✅ 单元测试覆盖率 > 70%
- ✅ 无重复代码
- ✅ 清晰的组件结构
- ✅ 完整的文档

---

**文档版本**: 1.0
**最后更新**: 2026-03-17
**下一步**: 开始 Phase 1 - 清理和准备
