# API Doc Plugin 设计文档

## 概述

复刻 Java 版本的 api-doc-plugin 到 Rust/Tauri 平台。核心功能：解析 Spring Boot JAR 包中的 Controller 类，读取 Swagger/Spring 注解，生成标准化的 API 接口文档。

## 需求确认

| 需求项 | 决策 |
|--------|------|
| 解析方式 | 纯 Rust 解析 .class 字节码 |
| 导出格式 | Word (.docx) + Markdown (.md) + HTML (.html) |
| HTTP 方法 | 全部 (GET/POST/PUT/DELETE/PATCH) |
| 前端布局 | 向导式三步 (配置 → 选择 → 导出) |
| Mock 数据 | 支持根据字段类型自动生成 |
| 依赖 JAR | 混合模式 (默认自动扫描 + 手动指定加速) |
| 循环引用 | 维护已访问集合防止无限递归 |
| Controller 发现 | 自动扫描 @Controller/@RestController，树形勾选选择 |

## 技术选型

| 功能 | Crate | 版本 |
|------|-------|------|
| .class 字节码解析 | `cafebabe` | 0.9 |
| Word 文档生成 | `docx-rs` | 0.4 |
| JAR/ZIP 解压 | `zip` | 2.x |
| 异步运行时 | `tokio` | 1.x (已有) |
| JSON 处理 | `serde` + `serde_json` | (已有) |

## 目录结构

```
plugins/api-doc/
├── Cargo.toml
├── manifest.json
├── assets/                       # 前端构建产物
├── frontend/                     # React + Vite
│   ├── src/
│   │   ├── main.tsx
│   │   ├── App.tsx               # 主组件 (三步向导)
│   │   ├── App.css
│   │   └── index.css
│   ├── vite.config.ts
│   ├── package.json
│   └── tsconfig.json
└── src/
    ├── lib.rs                    # Plugin trait 实现 + handle_call 路由
    ├── storage.rs                # 配置/历史持久化
    ├── models/
    │   ├── mod.rs
    │   ├── config.rs             # ApiDocConfig, ExportFormat, ExportConfig
    │   └── api_info.rs           # ApiInfo, ApiField, NodeInfo, ControllerInfo, MethodInfo
    ├── parser/
    │   ├── mod.rs                # JarParser 入口
    │   ├── class_parser.rs       # .class 文件解析 (cafebabe)
    │   ├── annotation.rs         # 注解匹配与提取
    │   ├── type_resolver.rs      # 泛型类型解析 + 循环引用防护
    │   └── mock.rs               # Mock 数据生成器
    └── exporter/
        ├── mod.rs                # DocumentExporter trait
        ├── word.rs               # Word 导出 (docx-rs)
        ├── markdown.rs           # Markdown 导出
        └── html.rs               # HTML 导出
```

## 核心解析逻辑

### 两阶段解析流程

**阶段 1 - Controller 发现** (scan_controllers)：

```
用户指定 JAR 路径
  → 解压 JAR (zip crate)
  → 扫描 BOOT-INF/classes/ 中所有 .class 文件
  → 解析每个 .class 的类级别注解
  → 过滤出带有 @Controller 或 @RestController 的类
  → 对每个 Controller 类，提取其方法的 HTTP 注解信息
  → 返回 Vec<ControllerInfo> (类名 + 类路径 + 方法列表)
```

前端展示为树形结构供用户勾选。

**阶段 2 - API 详情解析** (parse_api_details)：

```
用户选定的 Controller + 方法列表
  → 解析目标方法的参数类型和返回类型
  → 提取 @ApiOperation, @ApiModelProperty 等注解详情
  → 从 Signature 属性解析泛型参数类型
  → 递归解析请求/响应 DTO 的字段
    → BOOT-INF/classes/ 中查找自定义类
    → BOOT-INF/lib/ 的依赖 JAR 中查找 (自动扫描或用户指定)
    → 内置类型映射表处理 Java 基本类型和常用类
  → 生成 Mock 示例数据
  → 返回 Vec<ApiInfo>
```

### .class 文件注解匹配

通过 `cafebabe` 的 `AttributeData::RuntimeVisibleAnnotations` 提取注解：

| Java 注解 | type_descriptor |
|-----------|----------------|
| `@Controller` | `Lorg/springframework/stereotype/Controller;` |
| `@RestController` | `Lorg/springframework/web/bind/annotation/RestController;` |
| `@RequestMapping` | `Lorg/springframework/web/bind/annotation/RequestMapping;` |
| `@GetMapping` | `Lorg/springframework/web/bind/annotation/GetMapping;` |
| `@PostMapping` | `Lorg/springframework/web/bind/annotation/PostMapping;` |
| `@PutMapping` | `Lorg/springframework/web/bind/annotation/PutMapping;` |
| `@DeleteMapping` | `Lorg/springframework/web/bind/annotation/DeleteMapping;` |
| `@PatchMapping` | `Lorg/springframework/web/bind/annotation/PatchMapping;` |
| `@ApiOperation` | `Lio/swagger/annotations/ApiOperation;` |
| `@ApiModelProperty` | `Lio/swagger/annotations/ApiModelProperty;` |

注解的键值对从 `Annotation.elements` 中提取：
- `@ApiOperation(value="xxx")` → elements 中 key="value"
- `@ApiModelProperty(value="xxx", required=true, example="123")` → 对应 key-value 对
- `@PostMapping("/path")` → elements 中 key="value"，值为字符串数组

### 泛型类型解析

从方法的 `Signature` 属性或字段的 `Signature` 属性解析泛型信息：
- `Ljava/util/List<Lcom/example/User;>;` → List<User>
- `Ljava/util/Map<Ljava/lang/String;Ljava/lang/Object;>;` → Map<String, Object>

类型解析优先级：
1. 内置类型映射表 (Java primitives, String, Date, BigDecimal 等)
2. `BOOT-INF/classes/` 中的项目类
3. 自动扫描的 `BOOT-INF/lib/` 依赖 JAR / 用户指定的依赖 JAR
4. 找不到则标记为原始类型名，不递归解析

### 循环引用防护

解析 DTO 字段时维护 `HashSet<String>` (已访问的类全限定名)：
- 递归解析前检查类名是否已在集合中
- 已访问的类不再递归，字段类型标记为原始类型名
- 与 Java 版本 NodeInfo 的 equals/hashCode 去重逻辑一致

### Mock 数据生成规则

| Java 类型 | Mock 值 |
|-----------|---------|
| boolean/Boolean | `true` |
| int/Integer | `0` |
| long/Long | `0` |
| double/Double | `0.0` |
| float/Float | `0.0` |
| String | `"String"` |
| BigDecimal | `0` |
| Date/LocalDateTime | `"2024-01-01 00:00:00"` |
| LocalDate | `"2024-01-01"` |
| List/Set | 生成包含 2 个元素的数组 |
| 自定义对象 | 递归生成 |

优先使用 `@ApiModelProperty(example="xxx")` 的值。

## 数据模型

### ApiDocConfig (用户输入)

```rust
struct ApiDocConfig {
    source_jar_path: String,           // JAR 包路径
    service_name: String,              // 服务名称
    dependency_jars: Vec<String>,      // 依赖 JAR 名称前缀 (可选，用于加速)
    auto_scan_dependencies: bool,      // 是否自动扫描所有依赖 JAR (默认 true)
}
```

### ControllerInfo (Controller 发现结果)

```rust
struct ControllerInfo {
    class_name: String,                // 类全限定名
    class_path: String,                // 类级别 @RequestMapping 路径
    methods: Vec<MethodInfo>,          // 带有 HTTP 注解的方法列表
}

struct MethodInfo {
    method_name: String,               // Java 方法名
    http_method: String,               // GET/POST/PUT/DELETE/PATCH
    path: String,                      // 方法级别路径
    api_name: String,                  // @ApiOperation 值 (可能为空)
}
```

### ApiInfo (单个 API 接口完整信息)

```rust
struct ApiInfo {
    api_name: String,                  // 接口名称 (@ApiOperation)
    http_method: String,               // HTTP 方法 (GET/POST/PUT/DELETE/PATCH)
    service_name: String,              // 服务标识
    business_module: String,           // 业务模块 (@RequestMapping 首段)
    method_name: String,               // 方法路径
    version: String,                   // 版本号 (@RequestMapping 末段)
    full_path: String,                 // 完整路径
    req_fields: Vec<ApiField>,         // 请求参数
    req_example: String,               // 请求示例 JSON
    resp_nodes: Vec<NodeInfo>,         // 响应节点
    resp_example: String,              // 响应示例 JSON
}

struct ApiField {
    field_name: String,
    field_type: String,
    required: String,                  // "是" / "否"
    field_length: String,
    comment: String,
    example_value: String,
}

struct NodeInfo {
    node_name: String,
    node_desc: String,                 // 类简单名
    resp_fields: Vec<ApiField>,
}
```

## 导出功能

### Word (.docx)

使用 `docx-rs` 生成，文档结构与 Java 版本一致：

1. **接口说明** - 表格：业务场景/功能/调用范围/备注
2. **接口定义** - HTTP 方式 + 路径表格 (d/c/m/v)
3. **请求参数** - 表格：参数名/类型/必传/说明/备注
4. **请求示例** - 格式化 JSON（Consolas 等宽字体）
5. **响应参数** - 各 NodeInfo 字段表格 + 固定 errCode/alert 字段
6. **响应示例** - 成功响应 + 失败响应
7. **接口流程** - 空白章节
8. **接口依赖** - 空白章节

字体：标题用微软雅黑，正文用等线，代码用 Consolas。表头蓝色背景 (#8EAADB)。

### Markdown (.md)

标准 Markdown 格式，使用 MD 表格和代码块。

### HTML (.html)

带内联 CSS 的独立页面，样式与 Word 版本保持一致。

## 前端 UI 设计

### 向导式三步流程

**步骤 1 - 配置页** (`config` 视图)：
- JAR 包路径输入 + 文件选择按钮 (`.jar` 过滤)
- 服务名称文本输入
- 依赖 JAR 配置：
  - 自动扫描开关 (默认开启)
  - 手动指定依赖 JAR 名称前缀 (逗号分隔，仅在关闭自动扫描或需要加速时使用)
- 历史配置自动填充
- "扫描 Controller" 按钮

**步骤 2 - 选择与预览页** (`select` 视图)：
- Controller 树形列表：
  - 第一级：Controller 类名 + @RequestMapping 路径前缀 (可搜索过滤)
  - 第二级：该类中带 HTTP 注解的方法 (方法名 + HTTP 方法 + 路径 + @ApiOperation 描述)
  - 每级均有复选框，支持全选/反选
  - 搜索框：按类名/方法名/路径模糊搜索
- 选定后点击 "解析详情" 按钮
- 解析完成后展示 API 详情预览：
  - 每个 API 显示：接口名称、HTTP 方法、完整路径、请求参数表、响应字段表
- 导出格式选择 (Word/Markdown/HTML 复选框)
- 输出目录选择
- "导出" 按钮

**步骤 3 - 导出页** (`export` 视图)：
- 导出进度条
- 导出结果列表 (文件名 + 状态)
- "打开目录" 按钮
- "返回" 按钮 (回到配置页)

### 与后端通信

```typescript
// handle_call 方法路由
callAPI('save_config', { config })                       // 保存配置
callAPI('load_config')                                    // 加载配置
callAPI('scan_controllers', { jarPath })                  // 扫描 JAR 中的 Controller → Vec<ControllerInfo>
callAPI('parse_api_details', { jarPath, selectedMethods, serviceName, dependencyJars, autoScan })  // 解析详情 → Vec<ApiInfo>
callAPI('export_docs', { apis, formats, outputDir })      // 导出文档
callAPI('get_export_history')                             // 导出历史
```

## 与现有插件的一致性

- 遵循 `worktools-plugin-api` 的 `Plugin` trait
- 使用 `PluginStorage` 持久化数据到 `~/.worktools/history/plugins/api-doc.json`
- 前端通过 `window.pluginAPI.call()` 与后端通信
- `vite.config.ts` 输出固定文件名 (main.js / styles.css / index.html)
- `manifest.json` 格式与 db-doc 一致
- CSS 样式参考 db-doc 插件的配色和布局
