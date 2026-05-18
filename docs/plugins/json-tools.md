# JSON 工具（json-tools）

> JSON 格式化、压缩、转义/反转义及树形可视化编辑

## 功能特性

- JSON 格式化（美化缩进）与压缩（紧凑输出）
- 转义/反转义 JSON 字符串中的特殊字符（`\n`, `\t`, `\r`, `\"`, `\\`）
- 实时 JSON 语法验证，错误定位到行号/列号，并提供中文修复建议
- 树形视图：可展开/折叠/选中 JSON 节点，支持全展开、全折叠、删除选中节点
- 底部状态栏显示节点数量和数据大小

## 使用方法

### 基本操作

1. **输入 JSON** -- 在左侧编辑器面板中输入或粘贴 JSON 文本
2. **格式化** -- 点击工具栏"格式化"按钮，将 JSON 美化为 2 空格缩进格式（需要合法 JSON）
3. **压缩** -- 点击工具栏"压缩"按钮，移除所有多余空白（需要合法 JSON）
4. **转义** -- 点击"转义"按钮，将 JSON 中的特殊字符转义为 `\n`, `\"` 等转义序列
5. **反转义** -- 点击"去转义"按钮，将转义序列还原为原始字符
6. **树形浏览** -- 右侧面板实时显示解析后的树形结构，点击节点可选中，点击箭头可展开/折叠
7. **删除节点** -- 选中树形视图中的节点后，点击"删除选中"移除该节点
8. **全展开/全折叠** -- 快速展开或折叠树形视图中所有层级

### 错误提示

- 编辑器下方实时显示 JSON 语法错误
- 错误信息包含中文描述、行号、列号和修复建议（如"在该行末尾添加逗号"）
- 支持多浏览器错误格式解析（Chrome position、Firefox line/column、Edge 等）

## 技术实现

### 后端（Rust）

**模块结构**：
- `src/lib.rs` -- 插件主入口，实现 Plugin trait。这是项目中最简单的插件，无状态、无持久化

**核心设计**：
- `JsonTools` 为 unit-like struct（空结构体），不持有任何状态
- 所有 handle_call 方法均为纯函数：接收 JSON 字符串，返回处理结果
- 不使用 `PluginStorage`，不读写文件

**handle_call 方法列表**：

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `format_json` | `{json: string}` | `{result: string}` | 格式化 JSON（带缩进换行） |
| `minify_json` | `{json: string}` | `{result: string}` | 压缩 JSON（移除空白） |
| `escape_json` | `{json: string}` | `{result: string}` | 转义特殊字符（`\` `" ` `n` `r` `t`） |
| `unescape_json` | `{json: string}` | `{result: string}` | 反转义特殊字符 |
| `validate_json` | `{json: string}` | `{valid: boolean, error: string\|null}` | 验证 JSON 合法性 |

**数据存储**：无。所有操作为即时计算，不持久化任何数据。

**依赖的外部库**：

| crate | 用途 |
|-------|------|
| `serde_json` | JSON 解析、格式化、压缩 |
| `serde` | 序列化框架 |
| `worktools-plugin-api` | Plugin trait |

### 前端（React + TypeScript）

**组件结构**：

```
App.tsx
├── Toolbar              -- 工具栏（格式化/压缩/转义/去转义/全展开/全折叠/删除选中）
├── json-workspace
│   ├── JsonEditor       -- 左侧文本编辑器（textarea）
│   └── JsonTree         -- 右侧树形视图
│       └── TreeNode     -- 递归渲染的树节点
│           └── ValueNode -- 叶子节点值渲染（string/number/boolean/null）
└── json-statusbar       -- 底部状态栏（节点数、数据大小、格式化状态）
```

**核心工具函数**（`utils/`）：
- `jsonUtils.ts`：
  - `validateJson()` -- JSON 语法验证，返回 `ValidationError`（含行号/列号/中文建议）
  - `formatJson()` / `minifyJson()` -- 格式化/压缩（纯前端实现，不调用后端）
  - `escapeJson()` / `unescapeJson()` -- 转义/反转义
  - `parseErrorPosition()` -- 多浏览器错误位置解析
  - `findErrorPosition()` -- 手动分析缺少逗号等常见错误
- `treeUtils.ts`：
  - `getValueByPath()` / `deleteByPath()` -- 通过路径读写 JSON 对象
  - `expandAll()` -- 生成全展开状态映射

**pluginAPI.call 调用**：前端 JSON 处理全部在浏览器端完成（使用原生 `JSON.parse` / `JSON.stringify`），后端方法存在但前端未直接调用。前端工具栏操作直接操作本地 state。

**特殊处理**：
- 错误定位支持 Chrome（position）、Firefox（line/column）、Edge 等多浏览器格式
- 手动错误分析：检测对象属性间缺少逗号、对象/数组间缺少逗号等常见问题
- 错误信息翻译为中文，附带修复建议
- 树形视图选中路径通过 `JsonPath`（`Array<string | number>`）类型表示

**前端依赖**：
- React 18 + TypeScript + Vite 4
- 无额外第三方依赖

## 开发与调试

```bash
# Rust 后端
cargo check -p json-tools               # 类型检查
cargo test -p json-tools                 # 运行测试

# 前端
cd plugins/json-tools/frontend
npm run dev                              # 启动 Vite 开发服务器
npm run build                            # TypeScript 检查 + 构建
```

## 已知限制

- 前端的 JSON 处理（格式化、压缩等）完全在浏览器端完成，后端的对应方法目前未使用。如果未来需要处理超大 JSON 文件，应考虑将计算移到后端
- 转义/反转义基于字符串替换实现，可能对嵌套转义场景处理不够精确
- 树形视图对超大 JSON（数千节点）性能未做专项优化
- 错误位置检测的启发式规则仅覆盖缺少逗号等常见情况，复杂语法错误的定位可能不准确
