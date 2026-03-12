# JSON 工具插件设计文档

**创建日期**: 2026-03-05
**插件名称**: JSON 工具 (json-tools)
**插件图标**: { }
**插件版本**: 1.0.0

---

## 1. 插件概述

JSON 工具是一个基于 Tauri + Rust 的可扩展插件,提供 JSON 格式化、压缩、转义、去转义以及可视化编辑功能。

**核心功能**:
- 左侧工具栏: 格式化、压缩、转义、去转义
- 右侧操作栏: 全展开、全折叠、删除选中节点
- 双面板布局: 左侧文本编辑 + 右侧树形可视化
- 实时 JSON 语法验证和错误提示
- 临时工具模式,不持久化数据

---

## 2. 技术架构

### 2.1 后端 (Rust)

**文件结构**:
```
plugins/json-tools/
├── Cargo.toml
├── src/
│   └── lib.rs           # 插件实现,提供 JSON 处理方法
└── manifest.json        # 插件元数据
```

**核心方法**:
- `format_json`: 格式化 JSON (美化缩进)
- `minify_json`: 压缩 JSON (移除空白)
- `escape_json`: 转义 JSON (用于字符串嵌入)
- `unescape_json`: 去转义 JSON
- `validate_json`: 验证 JSON 语法,返回错误信息

**依赖库**:
- `serde_json`: JSON 解析和序列化
- `worktools-plugin-api`: 插件 API

### 2.2 前端 (React + TypeScript)

**文件结构**:
```
plugins/json-tools/frontend/
├── src/
│   ├── App.tsx          # 主组件,包含双面板布局
│   ├── components/
│   │   ├── JsonEditor.tsx   # 左侧文本编辑器
│   │   ├── JsonTree.tsx     # 右侧树形可视化
│   │   ├── Toolbar.tsx      # 左侧工具栏
│   │   └── TreeActions.tsx  # 右侧操作栏
│   └── utils/
│       ├── jsonUtils.ts # JSON 处理工具函数
│       └── treeUtils.ts # 树形操作工具函数
├── src/
│   ├── App.css          # 样式文件
│   └── main.tsx         # 入口文件
├── package.json
├── vite.config.ts
└── tsconfig.json
```

---

## 3. UI 设计规范

### 3.1 布局结构

```
┌─────────────────────────────────────────────────────────────┐
│  JSON 工具                                        [清空]    │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┬─────────────────────────────────┐ │
│  │  [✨格式化][📦压缩]   │  [📂全展开][📁全折叠][🗑️删除选中] │ │
│  │  [🔒转义][🔑去转义]  │                                 │ │
│  ├─────────────────────┼─────────────────────────────────┤ │
│  │                     │                                 │ │
│  │  左侧: 文本编辑器    │  右侧: 树形视图                 │ │
│  │                     │                                 │ │
│  │  {                  │  ▼ root                         │ │
│  │    "name": "test"   │    ▶ users (3)                  │ │
│  │  }                  │      "name": "John"             │ │
│  │                     │                                 │ │
│  └─────────────────────┴─────────────────────────────────┘ │
│                                                             │
│  ❌ 第 3 行, 第 5 列: Unexpected token...                  │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 图标配置

- **插件图标**: `{ }` (JSON 标识)
- **格式化**: ✨
- **压缩**: 📦
- **转义**: 🔒
- **去转义**: 🔑
- **全展开**: 📂
- **全折叠**: 📁
- **删除选中**: 🗑️

### 3.3 颜色方案

复用与密码管理器相同的 CSS 变量:
- `--accent`: #0078d4
- `--accent-light`: rgba(0, 120, 212, 0.1)
- `--bg-primary`: #ffffff
- `--bg-secondary`: #f5f5f5
- `--text-primary`: #1e1e1e
- `--text-secondary`: #7f8c8d
- `--border-color`: #e0e0e0

### 3.4 树形视图颜色

```css
.tree-key    { color: #d32f2f; }   /* 红色 - 键名 */
.tree-string { color: #2e7d32; }   /* 绿色 - 字符串 */
.tree-number { color: #1565c0; }   /* 蓝色 - 数字 */
.tree-boolean{ color: #c62828; }   /* 深红 - 布尔值 */
.tree-null   { color: #7f8c8d; }   /* 灰色 - null */
```

---

## 4. 数据流设计

```
用户输入 → JsonEditor (左侧)
  ↓
状态更新 (App.tsx: jsonText)
  ↓
实时验证 → jsonUtils.validateJson()
  ↓ (如果有效)
解析为对象 → JSON.parse()
  ↓
同步到 JsonTree (右侧)
  ↓
用户点击节点 → 选中状态更新
  ↓
点击删除按钮 → 删除节点 → 更新 jsonText → 同步到编辑器
```

**关键状态**:
```typescript
interface JsonToolState {
  jsonText: string;          // 当前 JSON 文本
  parsedData: any;           // 解析后的对象
  error: ValidationError | null;      // 错误信息
  selectedPath: JsonPath | null;      // 选中的节点路径
  isExpanded: Record<string, boolean>; // 节点展开状态
  successMessage: string;    // 成功提示消息
}

type JsonPath = Array<string | number>;
```

---

## 5. 用户交互流程

### 5.1 基本编辑流程
1. 用户在左侧编辑器输入/粘贴 JSON
2. 实时验证,如有错误显示红色错误提示
3. JSON 有效时,右侧自动更新树形视图
4. 用户可以:
   - 点击左侧工具栏按钮进行格式化/压缩等操作
   - 在右侧树形视图中点击选择节点
   - 点击"删除选中"删除节点

### 5.2 节点删除流程
1. 用户在右侧点击某个节点 (如 `users[0].name`)
2. 节点高亮显示,记录路径 `["users", 0, "name"]`
3. 用户点击"删除选中"按钮
4. 系统从 parsedData 中删除该路径
5. 重新序列化为 JSON 文本
6. 同步更新到左侧编辑器和右侧树形视图

### 5.3 展开/折叠流程
1. 默认状态: 根节点展开,子节点折叠
2. 用户点击"全展开" → 递归展开所有节点
3. 用户点击"全折叠" → 只保留根节点展开
4. 用户点击节点前的箭头 → 切换该节点的展开/折叠状态

---

## 6. 错误处理设计

### 6.1 JSON 语法错误
- **检测时机**: 用户输入时 (防抖 300ms)
- **显示方式**:
  - 底部红色错误提示条
  - 显示错误行号和列号
  - 左侧工具栏按钮禁用 (灰色)
- **错误信息格式**: "第 3 行,第 5 列: Unexpected token..."

### 6.2 操作错误
- **删除根节点**: 提示"无法删除根节点"
- **删除索引越界**: 提示"节点不存在,可能已被删除"
- **转义失败**: 提示"转义操作失败,请检查输入"

---

## 7. 工具函数 API

### 7.1 前端工具函数

**jsonUtils.ts**:
```typescript
interface ValidationError {
  valid: boolean;
  error: string | null;
  line?: number;
  column?: number;
}

validateJson(jsonStr: string): ValidationError
formatJson(jsonStr: string): string
minifyJson(jsonStr: string): string
escapeJson(jsonStr: string): string
unescapeJson(jsonStr: string): string
```

**treeUtils.ts**:
```typescript
type JsonPath = Array<string | number>;

getValueByPath(obj: any, path: JsonPath): any
deleteByPath(obj: any, path: JsonPath): any
getJsonPath(obj: any, target: any, path?: JsonPath): JsonPath | null
```

### 7.2 后端 API

```rust
// 所有方法接收 { json: string } 参数
// 返回 { result: string } 或 { valid: bool, error: string | null }

format_json(json: &str) -> Result<Value>
minify_json(json: &str) -> Result<Value>
escape_json(json: &str) -> Result<Value>
unescape_json(json: &str) -> Result<Value>
validate_json(json: &str) -> Result<Value>
```

---

## 8. 测试策略

### 8.1 单元测试

**后端测试** (Rust):
- 测试 JSON 格式化功能
- 测试 JSON 压缩功能
- 测试 JSON 转义/去转义
- 测试 JSON 验证

**前端测试** (TypeScript + Jest):
- 测试工具函数 (jsonUtils, treeUtils)
- 测试组件渲染
- 测试用户交互

### 8.2 集成测试

- 测试插件加载
- 测试前端-后端通信
- 测试完整的工作流程

---

## 9. 开发和构建流程

### 9.1 开发模式

```bash
# 1. 编译插件动态库
cd plugins/json-tools
cargo build

# 2. 启动前端开发服务器
cd frontend
npm run dev

# 3. 在另一个终端启动主应用
cd ../../../tauri-app
npm run tauri dev
```

### 9.2 生产构建

```bash
# 1. 构建插件
cd plugins/json-tools
cargo build --release

# 2. 构建前端
cd frontend
npm run build

# 3. 打包插件
cd ..
zip -r json-tools.wtplugin.zip \
  manifest.json \
  target/release/libjson_tools.dylib \
  frontend/dist/

# 4. 安装插件
mkdir -p ~/.worktools/plugins/json-tools
unzip json-tools.wtplugin.zip -d ~/.worktools/plugins/json-tools/
```

---

## 10. 关键设计决策

### 10.1 数据存储
- **选择**: 临时工具模式,不持久化数据
- **原因**: JSON 工具是即时处理工具,不需要保存历史记录

### 10.2 编辑器选择
- **选择**: 使用 `<textarea>` 而非第三方代码编辑器
- **原因**: 保持轻量级,减少依赖,避免复杂的集成问题

### 10.3 节点选择方式
- **选择**: 点击选择 + 删除按钮
- **原因**: 界面更清爽,操作更明确,符合截图设计

### 10.4 实时验证
- **选择**: 输入时防抖 300ms 验证
- **原因**: 平衡性能和用户体验,避免频繁验证导致卡顿

---

## 11. 与现有插件的协调

### 11.1 样式一致性
- 复用密码管理器的 CSS 变量
- 使用相同的按钮样式、Toast 提示样式
- 保持相同的间距、圆角、阴影等视觉规范

### 11.2 交互一致性
- 所有 onClick 事件添加 `preventDefault()` 和 `stopPropagation()`
- 使用相同的用户反馈方式 (Toast 提示)
- 保持相同的加载状态和错误处理模式

### 11.3 代码结构一致性
- 使用相同的文件组织方式
- 使用相同的 TypeScript 类型定义风格
- 使用相同的错误处理模式

---

**设计完成日期**: 2026-03-05
**文档版本**: 1.0.0
