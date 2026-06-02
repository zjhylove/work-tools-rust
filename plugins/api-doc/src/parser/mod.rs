pub mod annotation;
pub mod mock;
pub mod type_resolver;

use std::collections::{HashMap, HashSet};
use std::io::Read;

use anyhow::{Context, Result};
use tracing::info;
use zip::ZipArchive;

use crate::models::{ApiInfo, ControllerInfo};

/// 内层泛型类型的解析结果
struct ResolvedInnerType {
    short_name: String,
    fields: Vec<crate::models::ApiField>,
    nodes: Vec<crate::models::NodeInfo>,
}

/// JAR 包解析器
pub struct JarParser {
    /// 主 JAR 中的 class 文件: class_name (com/xxx/Foo) -> Vec<u8>
    classes: HashMap<String, Vec<u8>>,
    /// 依赖 JAR 中的 class 缓存
    dependency_classes: HashMap<String, Vec<u8>>,
}

impl JarParser {
    /// 从 JAR 文件路径创建解析器
    pub fn new(jar_path: &str) -> Result<Self> {
        let file = std::fs::File::open(jar_path)
            .with_context(|| format!("无法打开 JAR 文件: {}", jar_path))?;
        let mut archive =
            ZipArchive::new(file).with_context(|| format!("无法解析 JAR 文件: {}", jar_path))?;

        let mut classes = HashMap::new();

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

        info!(jar_path = %jar_path, class_count = classes.len(), "JAR 文件加载完成");

        Ok(Self {
            classes,
            dependency_classes: HashMap::new(),
        })
    }

    /// 加载依赖 JAR (从 BOOT-INF/lib/ 中)
    pub fn load_dependencies(
        &mut self,
        jar_path: &str,
        prefixes: &[String],
        auto_scan: bool,
    ) -> Result<()> {
        info!(jar_path = %jar_path, auto_scan, dep_count = %prefixes.len(), "开始加载依赖 JAR");
        let file = std::fs::File::open(jar_path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();

            if name.starts_with("BOOT-INF/lib/") && name.ends_with(".jar") {
                let jar_name = name.strip_prefix("BOOT-INF/lib/").unwrap().to_string();
                let should_load = auto_scan || prefixes.iter().any(|p| jar_name.starts_with(p));

                if should_load {
                    let mut jar_data = Vec::new();
                    file.read_to_end(&mut jar_data)?;
                    if let Ok(mut dep_archive) = ZipArchive::new(std::io::Cursor::new(jar_data)) {
                        for j in 0..dep_archive.len() {
                            let mut dep_file = dep_archive.by_index(j)?;
                            let dep_name = dep_file.name().to_string();
                            if dep_name.ends_with(".class") {
                                let class_name =
                                    dep_name.strip_suffix(".class").unwrap().to_string();
                                let mut data = Vec::new();
                                dep_file.read_to_end(&mut data)?;
                                self.dependency_classes.insert(class_name, data);
                            }
                        }
                    }
                }
            }
        }
        info!(count = self.dependency_classes.len(), "依赖 JAR 加载完成");
        Ok(())
    }

    /// 扫描所有 @Controller/@RestController 类
    pub fn scan_controllers(&self) -> Result<Vec<ControllerInfo>> {
        let mut controllers = Vec::new();

        for (class_name, class_data) in &self.classes {
            if let Ok(class_file) = cafebabe::parse_class(class_data) {
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

        controllers.sort_by(|a, b| a.class_name.cmp(&b.class_name));
        info!(
            total_classes = self.classes.len(),
            controller_count = controllers.len(),
            "Controller 扫描完成"
        );
        Ok(controllers)
    }

    /// 获取 class 原始字节数据
    pub fn get_class_data(&self, class_name: &str) -> Option<&[u8]> {
        let internal_name = class_name.replace('.', "/");
        self.classes
            .get(&internal_name)
            .or_else(|| self.dependency_classes.get(&internal_name))
            .map(|v| v.as_slice())
    }

    /// 解析 class 并执行闭包，确保生命周期正确
    pub fn with_class<F, R>(&self, class_name: &str, f: F) -> Result<R>
    where
        F: FnOnce(&cafebabe::ClassFile) -> R,
    {
        let data = self
            .get_class_data(class_name)
            .ok_or_else(|| anyhow::anyhow!("类文件未找到: {}", class_name))?;
        let class_file = cafebabe::parse_class(data)
            .map_err(|e| anyhow::anyhow!("解析 class 文件失败 {}: {:?}", class_name, e))?;
        Ok(f(&class_file))
    }

    pub fn class_exists(&self, class_name: &str) -> bool {
        let internal_name = class_name.replace('.', "/");
        self.classes.contains_key(&internal_name)
            || self.dependency_classes.contains_key(&internal_name)
    }

    /// 解析选中的 API 列表，生成完整的 ApiInfo
    pub fn parse_api_details(
        &self,
        controllers: &[ControllerInfo],
        selected: &[(String, String)], // (class_name, method_name)
        service_name: &str,
    ) -> Result<Vec<ApiInfo>> {
        let mut apis = Vec::new();

        // 构建 class_name -> ControllerInfo 的映射
        let ctrl_map: HashMap<&str, &ControllerInfo> = controllers
            .iter()
            .map(|c| (c.class_name.as_str(), c))
            .collect();

        for (class_name, method_name) in selected {
            let ctrl = match ctrl_map.get(class_name.as_str()) {
                Some(c) => c,
                None => continue,
            };

            let method_info = match ctrl.methods.iter().find(|m| m.method_name == *method_name) {
                Some(m) => m,
                None => continue,
            };

            let full_path = format!("{}{}", ctrl.class_path, method_info.path);
            let (business_module, version) = extract_path_segments(&full_path);

            // 获取请求参数和返回类型
            let (req_fields, mut req_nodes, resp_nodes) = self
                .with_class(class_name, |class_file| {
                    self.extract_method_fields(class_file, method_name, &mut HashSet::new())
                })?;

            // 动态设置 HrmsAppApi 节点的 d/c/m/v 示例值
            let api_path_method = full_path.rsplit('/').next().unwrap_or("").to_string();
            let api_version_num = version.clone();
            for node in &mut req_nodes {
                if node.node_name == "HrmsAppApi" {
                    for f in &mut node.resp_fields {
                        match f.field_name.as_str() {
                            "d" => f.example_value = service_name.to_string(),
                            "c" => f.example_value = business_module.clone(),
                            "m" => f.example_value = api_path_method.clone(),
                            "v" => f.example_value = api_version_num.clone(),
                            _ => {}
                        }
                    }
                }
            }

            let req_example = mock::generate_req_mock_json(&req_fields, &req_nodes);
            let resp_example = mock::generate_resp_mock_json(&resp_nodes);

            let api_name = if method_info.api_name.is_empty() {
                format!("{} - {}", method_info.http_method, full_path)
            } else {
                method_info.api_name.clone()
            };

            apis.push(ApiInfo {
                api_name,
                http_method: method_info.http_method.clone(),
                service_name: service_name.to_string(),
                business_module,
                method_name: method_name.clone(),
                version,
                full_path,
                req_fields,
                req_nodes,
                req_example,
                resp_nodes,
                resp_example,
            });
        }

        apis.sort_by(|a, b| a.full_path.cmp(&b.full_path));
        info!(count = apis.len(), "API 详情解析完成");
        Ok(apis)
    }

    /// 从 class 文件提取方法的请求参数和返回类型
    /// Returns (req_fields, req_nodes, resp_nodes)
    fn extract_method_fields(
        &self,
        class_file: &cafebabe::ClassFile,
        method_name: &str,
        visited: &mut HashSet<String>,
    ) -> (
        Vec<crate::models::ApiField>,
        Vec<crate::models::NodeInfo>,
        Vec<crate::models::NodeInfo>,
    ) {
        use cafebabe::attributes::AttributeData;

        for method in &class_file.methods {
            if method.name != method_name {
                continue;
            }

            let mut req_fields = Vec::new();
            let mut req_nodes = Vec::new();
            let mut resp_nodes = Vec::new();

            // 获取方法的泛型签名
            let signature = method.attributes.iter().find_map(|attr| {
                if let AttributeData::Signature(sig) = &attr.data {
                    Some(sig.to_string())
                } else {
                    None
                }
            });

            // ── 1. 解析响应结构 ──

            // 从签名获取返回类型完整链（从外到内，如 HrmsAppResponse → PageResponse → DTO）
            let resp_type_chain: Vec<String> = signature
                .as_ref()
                .map(|sig| type_resolver::extract_return_type_chain_from_signature(sig))
                .unwrap_or_default();

            if resp_type_chain.len() >= 2 {
                // ── 有 wrapper + 内层类型（支持多层嵌套泛型）──
                let wrapper = &resp_type_chain[0];
                let mut wrapper_visited = visited.clone();
                let (mut wrapper_fields, wrapper_nodes) =
                    type_resolver::extract_dto_fields(wrapper, self, &mut wrapper_visited);

                // 收集所有内层类型的 fields + nodes
                let mut resp_inner_visited = wrapper_visited;
                let mut inner_data: Vec<ResolvedInnerType> = Vec::new();

                for inner_type in &resp_type_chain[1..] {
                    if self.class_exists(inner_type)
                        && type_resolver::is_custom_type_private(inner_type)
                    {
                        let (inner_fields, inner_nodes) =
                            type_resolver::extract_dto_fields(
                                inner_type,
                                self,
                                &mut resp_inner_visited,
                            );
                        if !inner_fields.is_empty() {
                            inner_data.push(ResolvedInnerType {
                                short_name: inner_type.rsplit('.').next().unwrap_or(inner_type).to_string(),
                                fields: inner_fields,
                                nodes: inner_nodes,
                            });
                        }
                    }
                }

                // 逐层链接 + 推入 resp_nodes（单次遍历）
                // 将当前层的 Object 类型引用替换为下一层的具体类型
                for i in 0..inner_data.len() {
                    if i + 1 < inner_data.len() {
                        // 用 split_at_mut 避免 i 和 i+1 的借用冲突
                        let (left, right) = inner_data.split_at_mut(i + 1);
                        let next_short = &right[0].short_name;
                        for f in &mut left[i].fields {
                            if f.field_type == "Object" {
                                f.field_type = next_short.clone();
                            }
                            if let Some(ref mut info) = f.collection_info {
                                if info.element_type == "Object" {
                                    info.element_type = next_short.clone();
                                    f.field_type = format!("{}<{}>", info.container, next_short);
                                }
                            }
                        }
                    }
                    resp_nodes.push(crate::models::NodeInfo {
                        node_name: inner_data[i].short_name.clone(),
                        node_desc: String::new(),
                        resp_fields: std::mem::take(&mut inner_data[i].fields),
                    });
                    resp_nodes.extend(std::mem::take(&mut inner_data[i].nodes));
                }

                // Level 0 (wrapper) → Level 1: 将 wrapper 的泛型字段（优先 "data"，回退任意 Object）替换为紧邻内层类型
                if let Some(first) = inner_data.first() {
                    let data_field = wrapper_fields
                        .iter_mut()
                        .find(|f| f.field_name == "data" && f.field_type == "Object");
                    let data_field = if data_field.is_some() {
                        data_field
                    } else {
                        wrapper_fields
                            .iter_mut()
                            .find(|f| f.field_type == "Object")
                    };
                    if let Some(f) = data_field {
                        f.field_type = first.short_name.clone();
                    }
                }

                let wrapper_short = wrapper.rsplit('.').next().unwrap_or(wrapper).to_string();
                resp_nodes.insert(
                    0,
                    crate::models::NodeInfo {
                        node_name: wrapper_short,
                        node_desc: String::new(),
                        resp_fields: wrapper_fields,
                    },
                );
                resp_nodes.extend(wrapper_nodes);
            } else if resp_type_chain.len() == 1 {
                // ── 单层类型（wrapper 无泛型内层 或 普通 DTO）──
                let return_type = &resp_type_chain[0];
                if self.class_exists(return_type)
                    && type_resolver::is_custom_type_private(return_type)
                {
                    let (dto_fields, nodes) =
                        type_resolver::extract_dto_fields(return_type, self, visited);
                    if !dto_fields.is_empty() {
                        let short_name = return_type
                            .rsplit('.')
                            .next()
                            .unwrap_or(return_type)
                            .to_string();
                        resp_nodes.push(crate::models::NodeInfo {
                            node_name: short_name,
                            node_desc: String::new(),
                            resp_fields: dto_fields,
                        });
                    }
                    resp_nodes.extend(nodes);
                }
            } else {
                // ── 回退：从方法描述符获取返回类型 ──
                let return_type = type_resolver::get_return_type_from_descriptor(
                    &method.descriptor.to_string(),
                );

                if self.class_exists(&return_type)
                    && type_resolver::is_custom_type_private(&return_type)
                {
                    let (dto_fields, nodes) =
                        type_resolver::extract_dto_fields(&return_type, self, visited);
                    if !dto_fields.is_empty() {
                        let short_name = return_type
                            .rsplit('.')
                            .next()
                            .unwrap_or(&return_type)
                            .to_string();
                        resp_nodes.push(crate::models::NodeInfo {
                            node_name: short_name,
                            node_desc: String::new(),
                            resp_fields: dto_fields,
                        });
                    }
                    resp_nodes.extend(nodes);
                }
            }

            // ── 2. 解析请求结构 ──

            // 收集所有参数类型（wrapper + inner）
            let param_types: Vec<String> = if let Some(ref sig) = signature {
                type_resolver::extract_param_types_from_signature(sig)
                    .into_iter()
                    .filter(|p| !p.starts_with("java/"))
                    .map(|p| p.replace('/', "."))
                    .collect()
            } else {
                method
                    .descriptor
                    .parameters
                    .iter()
                    .map(|p| annotation::get_field_type_name(p))
                    .filter(|p| type_resolver::is_custom_type_private(p) && self.class_exists(p))
                    .collect()
            };

            // 找 wrapper 类型中的 data: Object 字段，替换为内层类型
            // wrapper 类型特点：有 api/common/data 字段
            let param_inner_type = {
                let mut inner = None;
                let mut wrapper = None;
                for pt in &param_types {
                    if self.class_exists(pt) && type_resolver::is_custom_type_private(pt) {
                        if let Some(data) = self.get_class_data(pt) {
                            if let Ok(cf) = cafebabe::parse_class(data) {
                                let has_data_object = cf.fields.iter().any(|f| {
                                    f.name == "data"
                                        && annotation::get_field_type_name(&f.descriptor)
                                            == "Object"
                                });
                                if has_data_object {
                                    wrapper = Some(pt.clone());
                                } else {
                                    inner = Some(pt.clone());
                                }
                            }
                        }
                    }
                }
                // 如果有一个 wrapper 和一个 inner，inner 就是 wrapper 中 data 的实际类型
                if wrapper.is_some() {
                    inner
                } else {
                    None
                }
            };

            // 分离 wrapper 类型和内层类型，wrapper 先处理
            let is_wrapper_type = |pt: &str| -> bool {
                if let Some(data) = self.get_class_data(pt) {
                    if let Ok(cf) = cafebabe::parse_class(data) {
                        return cf.fields.iter().any(|f| {
                            f.name == "data"
                                && annotation::get_field_type_name(&f.descriptor) == "Object"
                        });
                    }
                }
                false
            };

            // Pass 1: 处理 wrapper 类型
            for param_type in &param_types {
                if !self.class_exists(param_type)
                    || !type_resolver::is_custom_type_private(param_type)
                    || !is_wrapper_type(param_type)
                {
                    continue;
                }

                let (fields, nodes) = type_resolver::extract_dto_fields(param_type, self, visited);

                let mut processed_fields = fields.clone();
                if let Some(ref inner) = param_inner_type {
                    let inner_short = inner.rsplit('.').next().unwrap_or(inner);
                    for f in &mut processed_fields {
                        if f.field_name == "data" && f.field_type == "Object" {
                            f.field_type = inner_short.to_string();
                        }
                    }
                    // 先 extract 内层类型(此时 visited 中还没有它)，再 extract wrapper
                    let (inner_fields, inner_nodes) =
                        type_resolver::extract_dto_fields(inner, self, visited);
                    if !inner_fields.is_empty() {
                        req_nodes.push(crate::models::NodeInfo {
                            node_name: inner_short.to_string(),
                            node_desc: String::new(),
                            resp_fields: inner_fields,
                        });
                        req_nodes.extend(inner_nodes);
                    }
                }
                req_fields.extend(processed_fields);
                req_nodes.extend(nodes);
            }

            // Pass 2: 处理非 wrapper 类型（跳过已 visited 的，即已在 wrapper 中处理过的内层类型）
            for param_type in &param_types {
                if !self.class_exists(param_type)
                    || !type_resolver::is_custom_type_private(param_type)
                    || is_wrapper_type(param_type)
                    || visited.contains(param_type)
                {
                    continue;
                }

                let (fields, nodes) = type_resolver::extract_dto_fields(param_type, self, visited);
                req_fields.extend(fields);
                req_nodes.extend(nodes);
            }

            return (req_fields, req_nodes, resp_nodes);
        }

        (Vec::new(), Vec::new(), Vec::new())
    }
}

/// 从 URL 路径提取业务模块和版本信息
fn extract_path_segments(path: &str) -> (String, String) {
    let parts: Vec<&str> = path
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let business_module = parts.first().unwrap_or(&"").to_string();
    let version = parts
        .iter()
        .find(|s| s.starts_with('v') && s.len() <= 10)
        .unwrap_or(&"")
        .to_string();

    (business_module, version)
}
