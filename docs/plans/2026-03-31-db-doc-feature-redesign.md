# db-doc 功能设计梳理

> 日期: 2026-03-31
> 状态: 设计评审

## 概述

db-doc 插件当前已完成 Phase 1-2 的核心功能（MySQL/PostgreSQL 元数据提取、Markdown 导出、连接管理、表选择、预览）。本文档梳理现有功能设计问题，规划下一阶段的改进方向。

## 现有问题清单

| # | 问题 | 严重度 | 影响 |
|---|------|--------|------|
| 1 | Word/PDF 导出未实现，返回"即将推出"错误 | 高 | 核心功能缺失 |
| 2 | 前端导出格式/模板选择器未实现，硬编码为 markdown + detailed | 高 | 用户无法选择 |
| 3 | 导出 output_dir 为空字符串，后端无默认值处理 | 高 | 导出可能失败 |
| 4 | Enterprise 模板与 Detailed 完全相同，无实质区别 | 中 | 功能虚设 |
| 5 | 切换 db_type 时端口不会自动更新 | 中 | 体验差 |
| 6 | 连接测试无 loading 状态反馈 | 中 | 体验差 |
| 7 | 表列表无搜索/过滤功能 | 中 | 表多时不可用 |
| 8 | 无步骤导航和返回机制 | 中 | 流程不清晰 |
| 9 | 错误处理薄弱，只有 setError，无重试 | 低 | 体验差 |
| 10 | CSS 硬编码颜色，无变量系统 | 低 | 维护困难 |

## 设计决策

### D1: 模板简化为两种

**决策**: 去掉 Enterprise 模板，保留 Simple + Detailed。

- **Simple**: 字段名、类型、说明（三列），无索引信息
- **Detailed**: 完整字段表（6列: 字段名、类型、可空、主键、默认值、说明）+ 索引信息（索引名、列、唯一、类型）+ 表注释

**理由**: Enterprise 模板依赖外键关系、ER 图等高级特性，增加复杂度但价值有限。两种模板覆盖大部分场景。

**影响**: `models/connection.rs` 中的 `TemplateStyle` 枚举需修改，删除 `Enterprise` 变体。

### D2: 导出目标为本地文件

**决策**: 通过 Tauri 文件对话框让用户选择保存目录，后端在该目录下生成文件。

**文件名格式**: `数据库文档_<数据库名>_YYYYMMDD.<ext>`

**理由**: 最直观的用户体验，与桌面应用行为一致。

### D3: Word 导出用 quick-xml 手写 OOXML

**决策**: 用已引入的 quick-xml + zip 手动构建 DOCX XML，不引入重型 docx 生成库。

**理由**: 依赖轻量，完全可控。DOCX 结构相对固定（标题 + 表格），手动构建可行。

### D4: PDF 导出用 printpdf 直接绘制

**决策**: 用已引入的 printpdf 手动布局文本和表格。

**关键点**: 需要处理中文字体嵌入。macOS 使用系统字体 PingFang SC，需确认 printpdf 的字体加载机制。

### D5: 前端保持纯 HTML/CSS + 增强交互

**决策**: 不引入 UI 框架，在现有基础上优化交互体验。

**理由**: 插件前端需要轻量，避免框架带来的 bundle 体积和复杂度。

### D6: 密码加密保持现状

**决策**: 保持 AES-256 ECB 模式不变。

**理由**: 本地单机应用，威胁模型有限。ECB 的主要弱点（相同明文→相同密文）在此场景下影响很小。

## 导出管线设计

```
用户选择表 → 选择格式(MD/Word/PDF) + 模板(Simple/Detailed)
  → 前端弹出导出配置面板
  → 调用 Tauri 文件对话框选择保存目录
  → 后端提取表元数据
  → 根据 ExportFormat 选择 Exporter:
    ├── MarkdownExporter → 生成 .md
    ├── WordExporter     → 生成 .docx (quick-xml + zip)
    └── PDFExporter      → 生成 .pdf (printpdf)
  → 保存文件到用户选择的目录
  → 记录导出历史
  → 前端显示成功 Toast (含文件路径)
```

### Exporter 统一接口

```rust
trait DocumentExporter {
    fn export(&self, tables: &[TableInfo], config: &ExportConfig) -> Result<String, Box<dyn Error>>;
}
```

MarkdownExporter（现有）、WordExporter（新增）、PDFExporter（新增）均实现此 trait。

## 前端 UX 改进

### 步骤导航

```
步骤 1: 连接管理 ─→ 步骤 2: 选择表 ─→ 步骤 3: 预览 & 导出
```

- 顶部步骤条，高亮当前步骤
- 已完成步骤可点击回退
- 每步有"上一步"按钮

### 导出配置面板（步骤 3 弹出层）

- 格式选择: Markdown / Word / PDF（单选按钮组）
- 模板选择: 简洁 / 详细（单选按钮组）
- 文件名预览（自动生成，只读）
- "选择目录并导出"按钮 → 触发 Tauri 对话框 → 后端导出
- 导出中显示 loading 状态
- 成功/失败 Toast 提示

### 连接管理优化

- 切换 db_type 时自动更新默认端口 (MySQL→3306, PostgreSQL→5432)
- 测试连接按钮增加 loading 旋转动画
- 测试结果: 成功显示绿色 ✓，失败显示红色 ✗ + 具体错误信息
- 连接列表顶部增加搜索框（按名称过滤）

### 表选择增强

- 搜索框: 实时过滤表名
- 行数估算: 通过 `SHOW TABLE STATUS` (MySQL) / `pg_class.reltuples` (PostgreSQL) 获取
- 按前缀批量选择: 输入前缀（如 `t_`），一键选中所有匹配的表
- 反选按钮

### 错误处理

- API 调用统一 try/catch
- Toast 通知组件: 顶部滑入，3 秒后自动消失
- 错误类型区分: 连接失败 / 查询超时 / 导出错误

### CSS 变量系统

```css
:root {
  --color-primary: #1890ff;
  --color-primary-hover: #40a9ff;
  --color-success: #52c41a;
  --color-error: #ff4d4f;
  --color-warning: #faad14;
  --color-bg: #f5f5f5;
  --color-card: #ffffff;
  --color-text: #333333;
  --color-text-secondary: #999999;
  --color-border: #e8e8e8;
  --radius: 8px;
  --shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}
```

## 模型变更

### TemplateStyle 枚举修改

```rust
// 之前
pub enum TemplateStyle { Simple, Detailed, Enterprise }

// 之后
pub enum TemplateStyle { Simple, Detailed }
```

### ExportConfig 修改

确保 `output_dir` 字段有合理默认值或在导出时通过参数覆盖。

## 实施优先级

| 优先级 | 任务 | 依赖 |
|--------|------|------|
| P0 | 模板枚举简化（删除 Enterprise） | 无 |
| P0 | 导出配置面板前端 | 无 |
| P0 | Tauri 文件对话框集成 | 无 |
| P0 | Markdown 导出完整流程打通 | 导出配置面板 + 文件对话框 |
| P1 | 步骤导航 | 无 |
| P1 | 连接管理优化（端口、loading、搜索） | 无 |
| P1 | 表选择增强（搜索、行数、前缀选择） | 无 |
| P1 | CSS 变量系统 | 无 |
| P1 | Toast 通知组件 | 无 |
| P2 | Word 导出器实现 | Exporter trait 重构 |
| P2 | PDF 导出器实现 | Exporter trait 重构 + 中文字体 |

## 不做的事情

以下功能经评估后决定不实现:

- **连接分组/标签**: 连接数量通常不多，搜索足够
- **在线编辑表注释**: 超出文档工具范围
- **表数据预览**: 隐私风险，不属于文档生成
- **Handlebars 模板引擎集成**: 预设模板足够，自定义模板增加维护成本
- **密码加密升级**: 本地应用 ECB 足够
