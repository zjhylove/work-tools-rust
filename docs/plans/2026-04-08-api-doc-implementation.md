# API Doc Plugin Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the api-doc plugin that parses Spring Boot JAR files, extracts API information from .class bytecode annotations, and generates API documentation in Word/Markdown/HTML formats.

**Architecture:** cdylib plugin following workspace conventions. Two-phase parsing: (1) scan Controllers, (2) parse selected API details. Uses `cafebabe` for .class parsing, `docx-rs` for Word generation, `zip` for JAR extraction. React + Vite frontend with 3-step wizard UI matching db-doc style.

**Tech Stack:** Rust (cafebabe 0.9, docx-rs 0.4, zip 2.x, tokio, serde) | React 19 + TypeScript + Vite 5

---

### Task 1: Scaffold Plugin Structure

**Files:**
- Create: `plugins/api-doc/Cargo.toml`
- Create: `plugins/api-doc/manifest.json`
- Modify: `Cargo.toml` (add workspace member)

**Step 1: Create plugin Cargo.toml**

Create `plugins/api-doc/Cargo.toml`:

```toml
[package]
name = "api-doc"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
worktools-plugin-api = { path = "../../shared/plugin-api" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
tokio = { version = "1.0", features = ["rt-multi-thread"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4"] }
tracing = "0.1"
zip = "2"
tempfile = "3"
```

**Step 2: Create manifest.json**

Create `plugins/api-doc/manifest.json`:

```json
{
  "id": "api-doc",
  "name": "API文档",
  "description": "解析Spring Boot JAR包,自动生成API接口文档 (Word/Markdown/HTML)",
  "version": "1.0.0",
  "icon": "📄",
  "author": "Work Tools Team",
  "homepage": "https://github.com/worktools/api-doc",
  "files": {
    "macos": "libapi_doc.dylib",
    "linux": "libapi_doc.so",
    "windows": "api_doc.dll"
  },
  "assets": {
    "entry": "index.html"
  },
  "permissions": [
    "filesystem",
    "network"
  ]
}
```

**Step 3: Add workspace member**

Add `"plugins/api-doc"` to the `members` array in root `Cargo.toml`.

**Step 4: Verify compilation**

Run: `cargo check -p api-doc`
Expected: Error about missing `src/lib.rs` (expected at this point)

**Step 5: Commit**

```bash
git add plugins/api-doc/Cargo.toml plugins/api-doc/manifest.json Cargo.toml
git commit -m "feat(api-doc): scaffold plugin structure"
```

---

### Task 2: Data Models

**Files:**
- Create: `plugins/api-doc/src/lib.rs` (minimal skeleton)
- Create: `plugins/api-doc/src/models/mod.rs`
- Create: `plugins/api-doc/src/models/config.rs`
- Create: `plugins/api-doc/src/models/api_info.rs`

**Step 1: Create minimal lib.rs**

Create `plugins/api-doc/src/lib.rs`:

```rust
use anyhow::Result;
use serde_json::Value;
use tokio::runtime::Runtime;
use worktools_plugin_api::Plugin;

pub mod models;
pub mod storage;

/// API 文档生成插件
pub struct ApiDocPlugin {
    storage: storage::ApiDocStorage,
    runtime: Runtime,
}

impl ApiDocPlugin {
    pub fn new() -> Self {
        Self {
            storage: storage::ApiDocStorage::new(),
            runtime: Runtime::new().expect("Failed to create tokio runtime"),
        }
    }
}

impl Default for ApiDocPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ApiDocPlugin {
    fn id(&self) -> &str { "api-doc" }
    fn name(&self) -> &str { "API文档" }
    fn description(&self) -> &str { "解析Spring Boot JAR包,自动生成API接口文档" }
    fn version(&self) -> &str { "1.0.0" }
    fn icon(&self) -> &str { "📄" }

    fn get_view(&self) -> String {
        "<div>API文档生成器加载中...</div>".to_string()
    }

    fn handle_call(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let result = match method {
            _ => Err(anyhow::anyhow!("未知方法: {}", method)),
        };
        result.map_err(|e| e.into())
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(ApiDocPlugin::new()));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
```

**Step 2: Create models/mod.rs**

Create `plugins/api-doc/src/models/mod.rs`:

```rust
mod config;
mod api_info;

pub use config::{ApiDocConfig, ExportFormat, ExportConfig};
pub use api_info::{ApiInfo, ApiField, NodeInfo, ControllerInfo, MethodInfo};
```

**Step 3: Create models/config.rs**

Create `plugins/api-doc/src/models/config.rs` with `ApiDocConfig`, `ExportFormat` (Word/Markdown/HTML), `ExportConfig`, `ExportHistory`. Follow the exact pattern from db-doc's `models/connection.rs` for serde derive macros and Display impls.

Key structs:
- `ApiDocConfig { source_jar_path, service_name, dependency_jars: Vec<String>, auto_scan_dependencies: bool }`
- `ExportFormat` enum with `Word`, `Markdown`, `Html` — custom Serialize/Deserialize (same pattern as db-doc)
- `ExportConfig { selected_apis: Vec<SelectedApi>, output_dir: String, formats: Vec<ExportFormat> }`
- `SelectedApi { class_name: String, method_name: String }` — identifies which APIs to export
- `ExportHistory { id, service_name, api_count, formats, output_path, exported_at }`

**Step 4: Create models/api_info.rs**

Create `plugins/api-doc/src/models/api_info.rs` with all the data types from the design doc:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerInfo {
    pub class_name: String,
    pub class_path: String,
    pub methods: Vec<MethodInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    pub method_name: String,
    pub http_method: String,
    pub path: String,
    pub api_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    pub api_name: String,
    pub http_method: String,
    pub service_name: String,
    pub business_module: String,
    pub method_name: String,
    pub version: String,
    pub full_path: String,
    pub req_fields: Vec<ApiField>,
    pub req_example: String,
    pub resp_nodes: Vec<NodeInfo>,
    pub resp_example: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiField {
    pub field_name: String,
    pub field_type: String,
    pub required: String,
    pub field_length: String,
    pub comment: String,
    pub example_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_name: String,
    pub node_desc: String,
    pub resp_fields: Vec<ApiField>,
}
```

**Step 5: Verify compilation**

Run: `cargo check -p api-doc`
Expected: PASS (will error on missing `storage` module — create a placeholder next)

**Step 6: Commit**

```bash
git add plugins/api-doc/src/
git commit -m "feat(api-doc): add data models"
```

---

### Task 3: Storage Layer

**Files:**
- Create: `plugins/api-doc/src/storage.rs`

**Step 1: Create storage.rs**

Follow the exact pattern from `plugins/db-doc/src/storage.rs`. Create `plugins/api-doc/src/storage.rs`:

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use worktools_plugin_api::storage::PluginStorage;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ApiDocData {
    pub version: u32,
    pub last_config: Option<crate::models::ApiDocConfig>,
    pub export_history: Vec<crate::models::ExportHistory>,
}

impl ApiDocData {
    pub fn new() -> Self {
        Self { version: 1, last_config: None, export_history: Vec::new() }
    }
}

pub struct ApiDocStorage {
    storage: PluginStorage,
}

impl ApiDocStorage {
    pub fn new() -> Self {
        Self { storage: PluginStorage::new("api-doc", "api-doc.json") }
    }

    pub fn save_config(&self, config: &crate::models::ApiDocConfig) -> Result<()> {
        let mut data: ApiDocData = self.storage.load_json()?;
        data.last_config = Some(config.clone());
        self.storage.save_json(&data)
    }

    pub fn load_config(&self) -> Result<Option<crate::models::ApiDocConfig>> {
        let data: ApiDocData = self.storage.load_json()?;
        Ok(data.last_config)
    }

    pub fn add_export_history(&self, history: crate::models::ExportHistory) -> Result<()> {
        let mut data: ApiDocData = self.storage.load_json()?;
        data.export_history.push(history);
        if data.export_history.len() > 50 { data.export_history.remove(0); }
        self.storage.save_json(&data)
    }

    pub fn get_export_history(&self) -> Result<Vec<crate::models::ExportHistory>> {
        let data: ApiDocData = self.storage.load_json()?;
        Ok(data.export_history)
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check -p api-doc`
Expected: PASS

**Step 3: Commit**

```bash
git add plugins/api-doc/src/storage.rs
git commit -m "feat(api-doc): add storage layer"
```

---

### Task 4: .class Bytecode Parser (cafebabe)

**Files:**
- Create: `plugins/api-doc/src/parser/mod.rs`
- Create: `plugins/api-doc/src/parser/annotation.rs`

Add `cafebabe = "0.9"` to `Cargo.toml` dependencies first.

**Step 1: Add cafebabe dependency**

Add to `plugins/api-doc/Cargo.toml` `[dependencies]`:
```toml
cafebabe = "0.9"
```

**Step 2: Create parser/mod.rs**

Create `plugins/api-doc/src/parser/mod.rs` with the `JarParser` struct:

```rust
pub mod annotation;

use anyhow::{Result, Context};
use std::collections::HashMap;
use std::io::Read;
use zip::ZipArchive;
use cafebabe::ClassFile;
use crate::models::{ControllerInfo, MethodInfo};

/// JAR 包解析器
pub struct JarParser {
    /// 解压后的 class 文件缓存: class_name (com/xxx/Foo) -> Vec<u8>
    classes: HashMap<String, Vec<u8>>,
    /// 依赖 JAR 中的 class 缓存: class_name -> Vec<u8>
    dependency_classes: HashMap<String, Vec<u8>>,
}

impl JarParser {
    /// 从 JAR 文件路径创建解析器
    pub fn new(jar_path: &str) -> Result<Self> {
        let file = std::fs::File::open(jar_path)
            .with_context(|| format!("无法打开 JAR 文件: {}", jar_path))?;
        let mut archive = ZipArchive::new(file)
            .with_context(|| format!("无法解析 JAR 文件: {}", jar_path))?;

        let mut classes = HashMap::new();
        let mut dependency_classes = HashMap::new();

        // 提取 BOOT-INF/classes/ 中的 .class 文件
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();

            if name.starts_with("BOOT-INF/classes/") && name.ends_with(".class") {
                let class_name = name
                    .strip_prefix("BOOT-INF/classes/")
                    .unwrap()
                    .strip_suffix(".class")
                    .unwrap()
                    .to_string();
                let mut data = Vec::new();
                file.read_to_end(&mut data)?;
                classes.insert(class_name, data);
            }
        }

        Ok(Self { classes, dependency_classes })
    }

    /// 加载依赖 JAR (从 BOOT-INF/lib/ 中)
    pub fn load_dependencies(&mut self, jar_path: &str, prefixes: &[String], auto_scan: bool) -> Result<()> {
        let file = std::fs::File::open(jar_path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();

            if name.starts_with("BOOT-INF/lib/") && name.ends_with(".jar") {
                let jar_name = name.strip_prefix("BOOT-INF/lib/").unwrap().to_string();
                let should_load = auto_scan
                    || prefixes.iter().any(|p| jar_name.starts_with(p));

                if should_load {
                    let mut jar_data = Vec::new();
                    file.read_to_end(&mut jar_data)?;
                    // 解压依赖 JAR 中的 class 文件
                    if let Ok(mut dep_archive) = ZipArchive::new(std::io::Cursor::new(jar_data)) {
                        for j in 0..dep_archive.len() {
                            let mut dep_file = dep_archive.by_index(j)?;
                            let dep_name = dep_file.name().to_string();
                            if dep_name.ends_with(".class") {
                                let class_name = dep_name.strip_suffix(".class").unwrap().to_string();
                                let mut data = Vec::new();
                                dep_file.read_to_end(&mut data)?;
                                self.dependency_classes.insert(class_name, data);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// 扫描所有 @Controller/@RestController 类
    pub fn scan_controllers(&self) -> Result<Vec<ControllerInfo>> {
        let mut controllers = Vec::new();

        for (class_name, class_data) in &self.classes {
            if let Ok(class_file) = cafebabe::parse_class(&mut &class_data[..]) {
                if annotation::is_controller(&class_file) {
                    let class_path = annotation::get_class_request_mapping(&class_file);
                    let methods = annotation::get_http_methods(&class_file);
                    if !methods.is_empty() {
                        controllers.push(ControllerInfo {
                            class_name: class_name.replace('/', "."),
                            class_path,
                            methods,
                        });
                    }
                }
            }
        }

        // 按类名排序
        controllers.sort_by(|a, b| a.class_name.cmp(&b.class_name));
        Ok(controllers)
    }

    /// 解析 class 文件
    pub fn parse_class(&self, class_name: &str) -> Result<ClassFile> {
        let internal_name = class_name.replace('.', "/");
        let data = self.classes.get(&internal_name)
            .or_else(|| self.dependency_classes.get(&internal_name))
            .ok_or_else(|| anyhow::anyhow!("类文件未找到: {}", class_name))?;
        cafebabe::parse_class(&mut &data[..])
            .with_context(|| format!("解析 class 文件失败: {}", class_name))
    }

    /// 检查类是否存在
    pub fn class_exists(&self, class_name: &str) -> bool {
        let internal_name = class_name.replace('.', "/");
        self.classes.contains_key(&internal_name) || self.dependency_classes.contains_key(&internal_name)
    }
}
```

**Step 3: Create parser/annotation.rs**

Create `plugins/api-doc/src/parser/annotation.rs` with annotation matching functions:

```rust
use cafebabe::{ClassFile, AttributeData, Annotation};
use crate::models::MethodInfo;

// 注解描述符常量
const CONTROLLER: &str = "Lorg/springframework/stereotype/Controller;";
const REST_CONTROLLER: &str = "Lorg/springframework/web/bind/annotation/RestController;";
const REQUEST_MAPPING: &str = "Lorg/springframework/web/bind/annotation/RequestMapping;";
const GET_MAPPING: &str = "Lorg/springframework/web/bind/annotation/GetMapping;";
const POST_MAPPING: &str = "Lorg/springframework/web/bind/annotation/PostMapping;";
const PUT_MAPPING: &str = "Lorg/springframework/web/bind/annotation/PutMapping;";
const DELETE_MAPPING: &str = "Lorg/springframework/web/bind/annotation/DeleteMapping;";
const PATCH_MAPPING: &str = "Lorg/springframework/web/bind/annotation/PatchMapping;";
const API_OPERATION: &str = "Lio/swagger/annotations/ApiOperation;";
const API_MODEL_PROPERTY: &str = "Lio/swagger/annotations/ApiModelProperty;";

const HTTP_METHODS: &[&str] = &[GET_MAPPING, POST_MAPPING, PUT_MAPPING, DELETE_MAPPING, PATCH_MAPPING];

/// 检查类是否有 @Controller 或 @RestController
pub fn is_controller(class: &ClassFile) -> bool {
    has_class_annotation(class, CONTROLLER) || has_class_annotation(class, REST_CONTROLLER)
}

fn has_class_annotation(class: &ClassFile, descriptor: &str) -> bool {
    if let Some(AttributeData::RuntimeVisibleAnnotations(annotations)) =
        class.attributes.iter().find_map(|attr| {
            if attr.name == "RuntimeVisibleAnnotations" {
                Some(&attr.data)
            } else {
                None
            }
        })
    {
        annotations.iter().any(|a| a.type_descriptor == descriptor)
    } else {
        false
    }
}

/// 获取类级别 @RequestMapping 路径
pub fn get_class_request_mapping(class: &ClassFile) -> String {
    if let Some(AttributeData::RuntimeVisibleAnnotations(annotations)) =
        class.attributes.iter().find_map(|attr| {
            if attr.name == "RuntimeVisibleAnnotations" { Some(&attr.data) } else { None }
        })
    {
        for ann in annotations {
            if ann.type_descriptor == REQUEST_MAPPING {
                return get_annotation_string_value(ann, "value")
                    .or_else(|| get_annotation_string_value(ann, "path"))
                    .unwrap_or_default();
            }
        }
    }
    String::new()
}

/// 获取方法上的 HTTP 注解信息
pub fn get_http_methods(class: &ClassFile) -> Vec<MethodInfo> {
    let mut methods = Vec::new();

    for method in &class.methods {
        for attr in &method.attributes {
            if attr.name == "RuntimeVisibleAnnotations" {
                if let AttributeData::RuntimeVisibleAnnotations(annotations) = &attr.data {
                    for ann in annotations {
                        if let Some(http_method) = http_method_from_descriptor(&ann.type_descriptor) {
                            let path = get_annotation_string_value(ann, "value")
                                .or_else(|| get_annotation_string_value(ann, "path"))
                                .unwrap_or_default();
                            let api_name = get_api_operation_name(annotations);
                            methods.push(MethodInfo {
                                method_name: method.name.clone(),
                                http_method,
                                path,
                                api_name,
                            });
                        }
                    }
                }
            }
        }
    }

    methods
}

fn http_method_from_descriptor(descriptor: &str) -> Option<String> {
    match descriptor {
        GET_MAPPING => Some("GET".to_string()),
        POST_MAPPING => Some("POST".to_string()),
        PUT_MAPPING => Some("PUT".to_string()),
        DELETE_MAPPING => Some("DELETE".to_string()),
        PATCH_MAPPING => Some("PATCH".to_string()),
        _ => None,
    }
}

/// 从注解元素中获取字符串值
pub fn get_annotation_string_value(ann: &Annotation, key: &str) -> Option<String> {
    for element in &ann.elements {
        if element.name == key {
            // cafebabe 的 element_value 可能需要根据实际 API 调整
            // 这里假设能获取到字符串形式的值
            if let Some(s) = extract_string_from_element(&element.value) {
                return Some(s);
            }
        }
    }
    None
}

/// 从注解元素值中提取字符串
fn extract_string_from_element(value: &cafebabe::ElementValue) -> Option<String> {
    // cafebabe 的 ElementValue 枚举需要根据实际 API 处理
    // 通常注解的 value 可能是字符串常量或字符串数组
    match value {
        cafebabe::ElementValue::ConstValue(index) => {
            // 从常量池获取字符串
            Some(format!("const_{}", index))
        }
        cafebabe::ElementValue::EnumConstValue { type_name, const_name } => {
            Some(const_name.clone())
        }
        cafebabe::ElementValue::Array(values) => {
            // 取数组第一个元素
            values.first().and_then(|v| extract_string_from_element(v))
        }
        _ => None,
    }
}

/// 从方法注解中获取 @ApiOperation 的 value
fn get_api_operation_name(annotations: &[Annotation]) -> String {
    annotations.iter()
        .find(|a| a.type_descriptor == API_OPERATION)
        .and_then(|a| get_annotation_string_value(a, "value"))
        .unwrap_or_default()
}

/// 获取字段上的 @ApiModelProperty 信息
pub fn get_api_model_property(class: &ClassFile, field_name: &str) -> (String, String, String) {
    // Returns (comment, required, example_value)
    for field in &class.fields {
        if field.name == field_name {
            for attr in &field.attributes {
                if attr.name == "RuntimeVisibleAnnotations" {
                    if let AttributeData::RuntimeVisibleAnnotations(annotations) = &attr.data {
                        for ann in annotations {
                            if ann.type_descriptor == API_MODEL_PROPERTY {
                                let comment = get_annotation_string_value(ann, "value").unwrap_or_default();
                                let required = get_annotation_bool_value(ann, "required").map(|b| if b { "是" else { "否" }).unwrap_or("否").to_string();
                                let example = get_annotation_string_value(ann, "example").unwrap_or_default();
                                return (comment, required, example);
                            }
                        }
                    }
                }
            }
        }
    }
    (String::new(), "否".to_string(), String::new())
}

fn get_annotation_bool_value(ann: &Annotation, key: &str) -> Option<bool> {
    for element in &ann.elements {
        if element.name == key {
            // 根据实际 cafebabe API 提取布尔值
            // ElementValue 枚举的具体变体需要在实现时确认
        }
    }
    None
}

/// 获取类级别的 @RequestMapping 注解的完整路径数组
pub fn get_request_mapping_paths(class: &ClassFile) -> Vec<String> {
    if let Some(AttributeData::RuntimeVisibleAnnotations(annotations)) =
        class.attributes.iter().find_map(|attr| {
            if attr.name == "RuntimeVisibleAnnotations" { Some(&attr.data) } else { None }
        })
    {
        for ann in annotations {
            if ann.type_descriptor == REQUEST_MAPPING {
                if let Some(path) = get_annotation_string_value(ann, "value") {
                    return vec![path];
                }
            }
        }
    }
    Vec::new()
}
```

**Important:** The `cafebabe` crate's exact API for `ElementValue`, `Annotation.elements`, and constant pool access needs to be verified at implementation time. The code above shows the intended structure — actual field/variant names may differ. Read cafebabe source if compilation fails.

**Step 4: Verify compilation**

Run: `cargo check -p api-doc`
Expected: May have compile errors due to cafebabe API differences. Fix as needed.

**Step 5: Commit**

```bash
git add plugins/api-doc/src/parser/ plugins/api-doc/Cargo.toml
git commit -m "feat(api-doc): add .class bytecode parser with annotation extraction"
```

---

### Task 5: Type Resolver and Mock Generator

**Files:**
- Create: `plugins/api-doc/src/parser/type_resolver.rs`
- Create: `plugins/api-doc/src/parser/mock.rs`

**Step 1: Create type_resolver.rs**

Implements Java type name resolution, generic signature parsing, and recursive DTO field extraction with cycle detection.

Key functions:
- `resolve_java_type(descriptor: &str) -> String` — Maps JVM type descriptors to readable names (e.g. `Ljava/lang/String;` → `String`, `I` → `int`)
- `parse_descriptor_fields(descriptor: &str) -> Vec<String>` — Extracts types from method descriptors
- `parse_generic_signature(sig: &str) -> Vec<String>` — Parses generic type arguments from Signature attribute
- `extract_dto_fields(class_file: &ClassFile, parser: &JarParser, visited: &mut HashSet<String>) -> Result<(Vec<ApiField>, Vec<NodeInfo>)>` — Recursively extracts fields from a DTO class, building NodeInfo for complex types

**Step 2: Create mock.rs**

Implements mock data generation based on Java types. Key function:

- `generate_mock_json(fields: &[ApiField], nodes: &[NodeInfo]) -> String` — Generates JSON example string
- `mock_value_for_type(field_type: &str, example_value: &str) -> String` — Returns mock value for a single field, preferring example_value if non-empty

**Step 3: Verify compilation**

Run: `cargo check -p api-doc`

**Step 4: Commit**

```bash
git add plugins/api-doc/src/parser/type_resolver.rs plugins/api-doc/src/parser/mock.rs
git commit -m "feat(api-doc): add type resolver and mock data generator"
```

---

### Task 6: API Detail Parser (Glue Logic)

**Files:**
- Modify: `plugins/api-doc/src/parser/mod.rs` — Add `parse_api_details()` method

**Step 1: Add parse_api_details to JarParser**

This method takes selected `ControllerInfo + MethodInfo` pairs and produces full `ApiInfo` objects by:
1. Re-parsing the Controller class to get method parameter/return type descriptors
2. Using `type_resolver` to resolve types to readable names
3. Using `annotation` module to extract @ApiModelProperty details
4. Recursively resolving DTO fields
5. Using `mock` module to generate example JSON
6. Extracting path segments for business_module and version

**Step 2: Verify compilation**

Run: `cargo check -p api-doc`

**Step 3: Commit**

```bash
git add plugins/api-doc/src/parser/
git commit -m "feat(api-doc): add API detail parsing with full annotation resolution"
```

---

### Task 7: Document Exporters

**Files:**
- Create: `plugins/api-doc/src/exporter/mod.rs`
- Create: `plugins/api-doc/src/exporter/markdown.rs`
- Create: `plugins/api-doc/src/exporter/word.rs`
- Create: `plugins/api-doc/src/exporter/html.rs`

Add `docx-rs = "0.4"` to Cargo.toml dependencies.

**Step 1: Create exporter trait (mod.rs)**

```rust
pub mod markdown;
pub mod word;
pub mod html;

use anyhow::Result;
use crate::models::ApiInfo;

pub trait DocumentExporter {
    fn export(&self, apis: &[ApiInfo], output_dir: &str, service_name: &str) -> Result<Vec<String>>;
}
```

**Step 2: Create markdown.rs**

Simplest exporter. Generates one .md file per API with:
- Title (api_name)
- HTTP Method + Full Path
- Request parameters table
- Request example JSON code block
- Response parameters tables (per NodeInfo)
- Response example JSON code block

**Step 3: Create html.rs**

Similar to markdown but wraps in HTML with inline CSS. Uses the same color scheme (#8EAADB headers, etc.).

**Step 4: Create word.rs**

Uses `docx-rs` to generate Word documents matching the Java template structure. This is the most complex exporter — follow the FreeMarker template structure:
- 4 sections: 接口说明, 接口定义, 接口流程, 接口依赖
- Tables with blue header backgrounds
- JSON code blocks with Consolas font
- Chinese font support (SimSun/east_asia)

**Step 5: Verify compilation**

Run: `cargo check -p api-doc`

**Step 6: Commit**

```bash
git add plugins/api-doc/src/exporter/ plugins/api-doc/Cargo.toml
git commit -m "feat(api-doc): add document exporters (Word/Markdown/HTML)"
```

---

### Task 8: Wire Up handle_call

**Files:**
- Modify: `plugins/api-doc/src/lib.rs` — Add all handler methods and route them in handle_call

**Step 1: Add handler methods**

Add these methods to `ApiDocPlugin`:

- `handle_save_config(params) -> Result<Value>` — Saves config via storage
- `handle_load_config() -> Result<Value>` — Loads saved config
- `handle_scan_controllers(params) -> Result<Value>` — Creates JarParser, scans controllers, returns Vec<ControllerInfo>
- `handle_parse_api_details(params) -> Result<Value>` — Creates JarParser with dependencies, calls parse_api_details, returns Vec<ApiInfo>
- `handle_export_docs(params) -> Result<Value>` — Deserializes ExportConfig, dispatches to exporters, saves history
- `handle_get_export_history() -> Result<Value>` — Returns export history

**Step 2: Route in handle_call**

```rust
fn handle_call(&mut self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let result = match method {
        "save_config" => self.handle_save_config(params),
        "load_config" => self.handle_load_config(),
        "scan_controllers" => self.handle_scan_controllers(params),
        "parse_api_details" => self.handle_parse_api_details(params),
        "export_docs" => self.handle_export_docs(params),
        "get_export_history" => self.handle_get_export_history(),
        _ => Err(anyhow::anyhow!("未知方法: {}", method)),
    };
    result.map_err(|e| e.into())
}
```

**Step 3: Verify compilation**

Run: `cargo check -p api-doc`

**Step 4: Run tests**

Run: `cargo test -p api-doc`
Expected: PASS

**Step 5: Commit**

```bash
git add plugins/api-doc/src/lib.rs
git commit -m "feat(api-doc): wire up handle_call with all API methods"
```

---

### Task 9: Frontend Scaffold

**Files:**
- Create: `plugins/api-doc/frontend/package.json`
- Create: `plugins/api-doc/frontend/vite.config.ts`
- Create: `plugins/api-doc/frontend/tsconfig.json`
- Create: `plugins/api-doc/frontend/tsconfig.node.json`
- Create: `plugins/api-doc/frontend/index.html`
- Create: `plugins/api-doc/frontend/src/main.tsx`
- Create: `plugins/api-doc/frontend/src/index.css`

**Step 1: Copy and adapt from db-doc**

All these files are identical to db-doc equivalents except:
- `package.json`: name = `"api-doc-frontend"`
- `index.html`: title = `"API文档生成"`
- `index.css`: identical

Use the exact content from db-doc's frontend files (read in context above), with only name/title differences.

**Step 2: Install dependencies**

Run: `cd plugins/api-doc/frontend && npm install`

**Step 3: Commit**

```bash
git add plugins/api-doc/frontend/
git commit -m "feat(api-doc): scaffold frontend with React + Vite"
```

---

### Task 10: Frontend UI - App.tsx and App.css

**Files:**
- Create: `plugins/api-doc/frontend/src/App.tsx`
- Create: `plugins/api-doc/frontend/src/App.css`

**Step 1: Create App.tsx**

Three-step wizard component following db-doc patterns:

**Step 1 (config view):**
- JAR path input + file open button (use `window.pluginAPI` open_folder_dialog or similar file dialog)
- Service name input
- Dependency config: auto-scan toggle + manual prefix input
- "Scan Controllers" button → calls `scan_controllers` API

**Step 2 (select view):**
- Tree-structured list of Controller classes with expandable methods
- Search/filter input
- Checkboxes at both class and method levels
- "Select All" / "Deselect All" buttons
- After selection, "Parse Details" button → calls `parse_api_details` API
- Parsed results shown as expandable API cards (api_name, http_method, path, fields tables)
- Export format checkboxes (Word/Markdown/HTML)
- Output directory selector
- "Export" button

**Step 3 (export view):**
- Progress indicator
- Results list (filename + status)
- "Open Directory" button (if pluginAPI supports it)
- "Back" button

Use the same TypeScript patterns as db-doc:
- `declare global { interface Window { pluginAPI: {...} } }`
- `callAPI` helper function
- `useEffect` with `setInterval` polling for pluginAPI injection
- Toast notifications
- Error/loading states

**Step 2: Create App.css**

Copy from db-doc's `App.css` as base, then add styles for:
- Tree view (indented checkboxes, expand/collapse)
- API detail cards
- HTTP method badges (color-coded: GET=blue, POST=green, PUT=orange, DELETE=red, PATCH=purple)

**Step 3: Build frontend**

Run: `cd plugins/api-doc/frontend && npm run build`
Expected: `main.js`, `styles.css`, `index.html` in `plugins/api-doc/assets/`

**Step 4: TypeScript check**

Run: `cd plugins/api-doc/frontend && npx tsc --noEmit`
Expected: PASS

**Step 5: Commit**

```bash
git add plugins/api-doc/frontend/src/ plugins/api-doc/assets/
git commit -m "feat(api-doc): add frontend wizard UI"
```

---

### Task 11: Integration Test and Final Polish

**Files:**
- May modify any file for bug fixes

**Step 1: Full workspace check**

Run: `cargo fmt && cargo clippy --all-targets`
Fix any warnings.

**Step 2: Full workspace test**

Run: `cargo test`
Expected: All tests pass (existing + new)

**Step 3: Frontend type check**

Run: `cd plugins/api-doc/frontend && npx tsc --noEmit`
Expected: PASS

**Step 4: Build plugin**

Run: `cargo build -p api-doc --release`
Expected: `libapi_doc.dylib` (or .so/.dll) produced

**Step 5: Verify plugin registration**

Ensure `manifest.json` and `assets/` are in place. The plugin should be discoverable by the main app.

**Step 6: Final commit**

```bash
git add -A
git commit -m "feat(api-doc): complete api-doc plugin implementation"
```

---

## Execution Notes

- **Task 4 (cafebabe)** is the highest-risk task — the cafebabe crate has low documentation. Read its source code on GitHub (https://github.com/nickelc/cafebabe) before implementing annotation extraction.
- **Task 5 (type resolver)** needs careful handling of JVM type descriptors. Test with real Spring Boot JAR files.
- **Task 7 (Word exporter)** is complex but follows a clear template structure. Start with Markdown (simplest), then HTML, then Word.
- The `ElementValue` enum in cafebabe may have different variants than assumed — verify at implementation time.
- Test with a real Spring Boot JAR that uses Swagger annotations to validate the full pipeline.
