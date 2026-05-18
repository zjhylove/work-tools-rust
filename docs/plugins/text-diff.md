# 文本比对（text-diff）

> 实时并排文本比对，支持行级/字符级差异高亮、文件导入、差异统计与导航

## 功能特性

- 并排（side-by-side）文本比对，行级差异高亮
- 字符级差异高亮：对修改的行进行逐字对比，精确标记变化的字符
- 差异统计：实时显示新增、删除、修改行数
- 文件导入：通过文件选择器或拖放加载文本文件
- 预处理选项：忽略空白差异、忽略大小写差异
- 智能防抖：小文本（<100 行）实时计算，大文本自动 100ms 防抖
- 行号显示，自适应宽度

## 使用方法

### 基本操作

1. **输入文本** -- 在左侧面板输入或粘贴原始文本，在右侧面板输入或粘贴修改后的文本
2. **导入文件** -- 点击顶部工具栏"原始"或"修改"按钮，通过文件选择器加载文件
3. **查看差异** -- 面板自动实时计算并高亮差异：红色背景为删除行，绿色背景为新增行，修改行内字符级高亮
4. **差异统计** -- 工具栏中间实时显示 `+N 新增`、`-N 删除`、`~N 修改` 统计
5. **拖放导入** -- 直接拖放文件到对应面板区域

### 配置项

通过后端 `preprocess_text` 方法提供（前端目前未暴露 UI 控件）：

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| ignore_whitespace | boolean | false | 忽略空白字符差异（将连续空白合并为单个空格） |
| ignore_case | boolean | false | 忽略大小写差异（转为小写后比较） |

## 技术实现

### 后端（Rust）

**模块结构**：
- `src/lib.rs` -- 插件主入口，包含文本比对核心逻辑和 Plugin trait 实现

**核心数据结构**：

| 结构体 | 用途 |
|--------|------|
| `TextFileContent` | 文件内容（content + encoding） |
| `ProcessOptions` | 预处理选项（ignore_whitespace, ignore_case） |
| `DiffStats` | 差异统计（additions, deletions, modifications） |

**比对算法**：
- 使用 `similar` crate 的 Patience 算法（擅长处理代码差异，比默认 Myers 算法产生更可读的结果）
- 统计逻辑：新增和删除的重叠部分记为"修改"（`modifications = min(additions, deletions)`）
- Unified Diff 导出：生成标准 `--- a/file` / `+++ b/file` / `@@ ... @@` 格式

**handle_call 方法列表**：

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `load_text_file` | `{file_path: string}` | `{content: string, encoding: string}` | 加载文本文件（最大 10MB） |
| `save_text_file` | `{file_path: string, content: string}` | `{success: true}` | 保存文本文件 |
| `preprocess_text` | `{text: string, ignore_whitespace?: bool, ignore_case?: bool}` | `{original: string, processed: string}` | 文本预处理 |
| `count_diff` | `{original: string, modified: string}` | `{additions, deletions, modifications}` | 差异统计 |
| `export_diff` | `{original: string, modified: string, filename?: string}` | `{diff: string}` | 导出 Unified Diff 格式 |

**数据存储**：无。不使用 `PluginStorage`，所有操作为即时计算。

**文件限制**：单个文件最大 10MB（`load_text_file_impl` 中检查文件 metadata）。

**依赖的外部库**：

| crate | 用途 |
|-------|------|
| `similar` | 文本差异比对引擎（Patience/Myers/LCS 算法） |
| `serde` / `serde_json` | JSON 序列化 |
| `worktools-plugin-api` | Plugin trait |

### 前端（React + TypeScript）

**组件结构**：

```
App.tsx
├── Toolbar                      -- 工具栏（原始文件/修改文件选择 + 差异统计）
├── editor-container
│   ├── EditorPane (left)        -- 左侧原始文本编辑面板
│   └── EditorPane (right)       -- 右侧修改后文本编辑面板
└── status-bar                   -- 底部状态栏

hooks/
├── useDiff.ts                   -- 核心差异计算 Hook（行级 + 字符级 diff）
├── useDiffNavigation.ts         -- 差异导航 Hook（上一个/下一个差异跳转）
├── useSyncScroll.ts             -- 双面板滚动同步 Hook
└── useDebounce.ts               -- 防抖 Hook（值防抖 + 回调防抖）

components/
├── EditorPane.tsx               -- 编辑面板（行号 + textarea + 高亮层叠加）
├── InlineDiffView.tsx           -- 行内差异视图（合并展示）
└── FilePickerButton.tsx         -- 文件选择按钮（含拖放支持）
```

**核心 Hook -- `useDiff`**：
- 使用 `diff` npm 包的 `diffLines`（行级对比）和 `diffChars`（字符级对比）
- 流程：`diffLines` 得到行级变更 -> 对齐两侧行 -> 对删除+新增配对的行执行 `diffChars` 字符级对比
- 统计：分别统计 additions、deletions，配对成功的记为 modifications
- 智能防抖：行数 <100 时延迟 0ms（实时），>=100 时延迟 100ms

**EditorPane 组件**：
- 三层叠加架构：行号层 + 高亮层（highlight-layer）+ textarea 输入层
- 高亮层使用 `pointer-events: none`，事件穿透到 textarea
- 行号通过 `transform: translateY()` 跟随滚动，不随内容重绘
- 行号宽度根据行数自适应（2 位数/3 位数等）
- 内容变化时通过 `requestAnimationFrame` 同步各层尺寸

**pluginAPI.call 调用列表**：

| pluginId | method | 说明 |
|----------|--------|------|
| `text-diff` | `load_text_file` | 通过后端加载文件（FilePickerButton 中使用） |

**注意**：主界面的差异计算完全在前端完成（使用 `diff` npm 包），不调用后端。后端方法（`count_diff`、`export_diff`、`preprocess_text`）保留供其他插件或未来功能使用。

**前端依赖**：

| 包 | 用途 |
|----|------|
| `diff` | 行级和字符级文本差异算法（前端核心依赖） |
| `@types/diff` | TypeScript 类型定义 |

## 开发与调试

```bash
# Rust 后端
cargo check -p text-diff                 # 类型检查
cargo test -p text-diff                   # 运行测试

# 前端
cd plugins/text-diff/frontend
npm run dev                               # 启动 Vite 开发服务器
npm run build                             # TypeScript 检查 + 构建
```

## 已知限制

- 后端的 `count_diff`、`export_diff`、`preprocess_text` 方法已实现但前端未暴露 UI 入口。前端差异计算完全在浏览器端完成
- `FilePickerButton` 组件通过 `prompt()` 输入文件路径，在 iframe 环境中可能受限。主界面的 `App.tsx` 使用原生 `<input type="file">` 替代，体验更好
- `useDiffNavigation` Hook 已实现差异索引和跳转逻辑，但未在主界面中接入（`DiffEditor.tsx` 中的 `goToDiff` 标记为 TODO）
- 前端 `DiffEditor.tsx` 组件存在但未在 `App.tsx` 中使用，实际差异展示通过 `useDiff` Hook + `EditorPane` 组件实现
- 字符级差异高亮对超长单行（如 minified JSON）性能未做专项优化
- 文件编码仅支持 UTF-8（`load_text_file_impl` 使用 `read_to_string`）
