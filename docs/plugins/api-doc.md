# API 文档生成（api-doc）

> 解析 Spring Boot JAR 包，自动生成 API 接口文档（Markdown/HTML）

## 功能特性

- Spring Boot JAR 包解析：自动扫描 `BOOT-INF/classes/` 下的 Controller 类
- Controller 发现：识别 `@Controller` / `@RestController` 注解，提取 `@RequestMapping` 路径
- HTTP 方法识别：解析 `@GetMapping` / `@PostMapping` / `@PutMapping` / `@DeleteMapping` / `@PatchMapping` 等注解
- API 详情解析：提取请求参数类型、响应类型，递归解析嵌套 DTO 字段
- Wrapper 类型支持：自动识别 `data: Object` 包装模式，解析内层实际类型
- 依赖 JAR 加载：支持从 `BOOT-INF/lib/` 加载依赖 JAR 解析 DTO 定义
- 多格式导出：Markdown、HTML 两种导出格式
- 向导式流程：配置 -> 选择 Controller -> 预览 -> 导出

## 使用方法

### 基本操作

1. **配置步骤**：
   - 点击"选择 JAR 文件"打开文件选择器，选取 Spring Boot Fat JAR
   - 输入服务名称（用于文档标题和示例值生成）
   - 点击"开始扫描"

2. **选择步骤**：
   - 扫描完成后显示所有 Controller 及其方法列表
   - 按类展开/折叠，勾选需要生成文档的 API
   - 支持全选/取消全选、搜索过滤
   - 点击"解析详情"获取完整的参数和返回值信息

3. **预览与导出**：
   - 查看解析后的 API 详情（请求参数、响应结构、示例 JSON）
   - 选择导出格式（Markdown / HTML）
   - 选择输出目录
   - 点击"导出"生成文档文件

### 配置项

- **source_jar_path**：Spring Boot JAR 文件路径
- **service_name**：服务名称（用于文档标题和 HrmsAppApi 节点的 d/c/m/v 示例值）
- **output_dir**：文档输出目录
- **formats**：导出格式，支持 `markdown` 和 `html`

## 技术实现

### 后端（Rust）

- **模块结构**：
  - `lib.rs` - 主入口，`ApiDocPlugin` struct，handle_call 方法分发
  - `storage.rs` - 配置持久化（`ApiDocStorage`）
  - `models/` - 数据模型
    - `config.rs` - `ApiDocConfig`、`ExportConfig`、`ExportFormat`、`SelectedApi`
    - `api_info.rs` - `ControllerInfo`、`MethodInfo`、`ApiInfo`、`ApiField`、`NodeInfo`
  - `parser/` - JAR 解析引擎
    - `mod.rs` - `JarParser`，JAR 加载、Controller 扫描、API 详情解析
    - `annotation.rs` - Java 注解解析（`@Controller`、`@RequestMapping` 等）
    - `type_resolver.rs` - Java 类型解析、DTO 字段提取、泛型签名解析
    - `mock.rs` - 请求/响应示例 JSON 生成
  - `exporter/` - 文档导出
    - `mod.rs` - `DocumentExporter` trait
    - `markdown.rs` - Markdown 导出
    - `html.rs` - HTML 导出

- **核心结构**：`ApiDocPlugin` 持有 `ApiDocStorage`

- **handle_call 方法列表**：

| 方法 | 参数 | 返回值 |
|---|---|---|
| `save_config` | `ApiDocConfig` (source_jar_path, service_name) | `{ success }` |
| `load_config` | (无) | `ApiDocConfig | null` |
| `scan_controllers` | `source_jar_path` | `[ControllerInfo, ...]` |
| `parse_api_details` | `source_jar_path`, `service_name?`, `controllers`, `selected: [[class_name, method_name]]` | `[ApiInfo, ...]` |
| `export_docs` | `apis`, `service_name?`, `output_dir`, `formats` | `[file_path, ...]` |

- **JAR 解析流程**：
  1. 使用 `zip` crate 解压 JAR 文件
  2. 从 `BOOT-INF/classes/` 提取 `.class` 文件
  3. 使用 `cafebabe` crate 解析 Java class 文件格式
  4. 扫描带有 `@Controller`/`@RestController` 的类
  5. 提取类级别 `@RequestMapping` 路径和方法级别 HTTP 注解
  6. 解析方法签名中的泛型参数类型和返回值类型
  7. 递归解析 DTO 类的字段（支持嵌套类型、Wrapper 模式）

- **类型解析策略**：
  - 通过方法签名（`Signature` attribute）提取泛型参数和返回类型
  - 识别 Wrapper 模式（包含 `data: Object` 字段的类）并提取内层类型
  - 使用 `visited` 集合防止循环引用
  - 从 URL 路径自动提取业务模块（第一段）和版本号（`v` 前缀段）

- **数据存储**：使用 `PluginStorage` 持久化到 `~/.worktools/history/plugins/api-doc.json`，保存最后使用的配置

- **依赖库**：
  - `zip` 2 - JAR（ZIP）文件解压
  - `cafebabe` 0.9 - Java class 文件格式解析
  - `tempfile` 3 - 临时文件处理
  - `chrono` 0.4 - 日期时间（文档时间戳）
  - `serde_json` / `serde` - JSON 序列化
  - `anyhow` - 错误处理
  - `tracing` 0.1 - 日志
  - `worktools-plugin-api` - 插件 trait + PluginStorage

### 前端（React + TypeScript）

- **组件结构**：
  - `App.tsx` - 根组件，管理三步向导流程状态
  - `StepHeader.tsx` - 步骤指示器（配置 -> 选择 -> 预览）
  - `ConfigView.tsx` - 配置页面（JAR 文件选择、服务名称输入）
  - `SelectView.tsx` - Controller 选择页面（树形勾选、搜索、解析）
  - `PreviewView.tsx` - 预览页面（导出结果展示）
  - `ApiCard.tsx` - API 详情卡片
  - `ControllerPanel.tsx` - Controller 面板
  - `DetailPanel.tsx` - API 详情面板
  - `ExportPanel.tsx` - 导出配置面板

- **类型定义**（`types.ts`）：
  - `ApiDocConfig` - 配置（source_jar_path, service_name）
  - `ControllerInfo` - Controller 信息（class_name, class_path, methods）
  - `MethodInfo` - 方法信息（method_name, http_method, path, api_name）
  - `ApiInfo` - 完整 API 信息（含请求/响应字段、节点、示例）
  - `ApiField` - 字段信息（name, type, required, length, comment, example）
  - `NodeInfo` - 嵌套节点信息（node_name, node_desc, resp_fields）
  - `ViewMode` - 视图模式（config / select / preview）

- **pluginAPI 调用**：
  - `api-doc` / `save_config` - 保存配置
  - `api-doc` / `load_config` - 加载上次配置
  - `api-doc` / `scan_controllers` - 扫描 JAR 中的 Controller
  - `api-doc` / `parse_api_details` - 解析选中 API 的详情
  - `api-doc` / `export_docs` - 导出文档
  - `pluginAPI.open_file_dialog` - 选择 JAR 文件
  - `pluginAPI.open_folder_dialog` - 选择输出目录

- **特殊依赖**：无第三方 UI 库
- **API 就绪检测**：使用 `setInterval` 轮询 `window.pluginAPI` 是否注入完成

## 开发与调试

```bash
# Rust 后端
cargo check -p api-doc
cargo test -p api-doc

# 前端
cd plugins/api-doc/frontend && npm run dev

# 完整构建
cd plugins/api-doc/frontend && npm run build
cargo build --release -p api-doc
```

## 已知限制

- 仅支持 Spring Boot Fat JAR 格式（`BOOT-INF/classes/` 结构）
- 不支持 WAR 包或普通 JAR 包
- 类型解析依赖 Java class 文件的 `Signature` attribute，如果编译时未保留泛型签名则无法解析实际类型
- `Map`、`List` 等集合类型的元素类型解析有限
- 不支持 Kotlin 编译的 class 文件中的特殊结构
- 依赖 JAR 自动加载可能耗时较长（大型项目）
- 不支持 Word/PDF 导出（仅 Markdown 和 HTML）
