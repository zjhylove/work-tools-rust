use std::collections::HashSet;
use std::ops::Deref;

use crate::models::{ApiField, NodeInfo};
use crate::parser::annotation;
use crate::parser::JarParser;

fn resolve_base_type(descriptor_char: char) -> &'static str {
    match descriptor_char {
        'B' => "byte",
        'C' => "char",
        'D' => "Double",
        'F' => "Float",
        'I' => "Integer",
        'J' => "Long",
        'S' => "Short",
        'Z' => "Boolean",
        _ => "Object",
    }
}

/// 解析方法描述符中的参数类型
/// 例如 `(Ljava/lang/String;I)V` -> ["String", "Integer"]
pub fn parse_descriptor_params(descriptor: &str) -> Vec<String> {
    let mut params = Vec::new();
    let chars: Vec<char> = descriptor.chars().collect();
    let mut i = 0;

    // 跳过 '('
    while i < chars.len() && chars[i] != '(' {
        i += 1;
    }
    if i < chars.len() {
        i += 1; // skip '('
    }

    while i < chars.len() && chars[i] != ')' {
        match chars[i] {
            'L' => {
                // 对象类型: Ljava/lang/String;
                let start = i + 1;
                while i < chars.len() && chars[i] != ';' {
                    i += 1;
                }
                let class_path: String = chars[start..i].iter().collect();
                let type_name = simplify_class_name(&class_path);
                params.push(type_name);
                i += 1; // skip ';'
            }
            '[' => {
                // 数组维度
                let mut dims = 0;
                while i < chars.len() && chars[i] == '[' {
                    dims += 1;
                    i += 1;
                }
                // 下一字符是元素类型
                let element_type = if i < chars.len() && chars[i] == 'L' {
                    let start = i + 1;
                    while i < chars.len() && chars[i] != ';' {
                        i += 1;
                    }
                    let class_path: String = chars[start..i].iter().collect();
                    simplify_class_name(&class_path)
                } else if i < chars.len() {
                    resolve_base_type(chars[i]).to_string()
                } else {
                    "Object".to_string()
                };
                i += 1;
                let brackets = "[]".repeat(dims);
                params.push(format!("{}{}", element_type, brackets));
            }
            c => {
                params.push(resolve_base_type(c).to_string());
                i += 1;
            }
        }
    }

    params
}

/// 获取类型的简短显示名: com.xxx.Foo -> Foo, String -> String
pub fn short_type_name(full_name: &str) -> &str {
    full_name.rsplit('.').next().unwrap_or(full_name)
}

/// 简化类名: java/lang/String -> String, com/xxx/Foo -> com.xxx.Foo
pub fn simplify_class_name(internal_name: &str) -> String {
    match internal_name {
        "java/lang/String" => "String".to_string(),
        "java/lang/Integer" => "Integer".to_string(),
        "java/lang/Long" => "Long".to_string(),
        "java/lang/Double" => "Double".to_string(),
        "java/lang/Float" => "Float".to_string(),
        "java/lang/Boolean" => "Boolean".to_string(),
        "java/lang/Object" => "Object".to_string(),
        "java/lang/Void" => "Object".to_string(),
        "java/lang/Date" => "Date".to_string(),
        "java/time/LocalDateTime" => "LocalDateTime".to_string(),
        "java/time/LocalDate" => "LocalDate".to_string(),
        "java/math/BigDecimal" => "BigDecimal".to_string(),
        "java/util/List" => "List".to_string(),
        "java/util/Map" => "Map".to_string(),
        "java/util/Set" => "Set".to_string(),
        _ => internal_name.replace('/', "."),
    }
}

/// 解析泛型签名，提取类型参数
/// 例如 `<T:Ljava/lang/Object;>Ljava/util/ArrayList<TT;>;` -> ["T"]
pub fn parse_generic_types(signature: &str) -> Vec<String> {
    let mut types = Vec::new();
    let chars: Vec<char> = signature.chars().collect();
    let mut i = 0;

    // 查找 < ... > 泛型参数部分
    if i < chars.len() && chars[i] == '<' {
        i += 1;
        while i < chars.len() && chars[i] != '>' {
            // 跳过类型变量名 (如 T)
            let name_start = i;
            while i < chars.len() && chars[i] != ':' {
                i += 1;
            }
            let name: String = chars[name_start..i].iter().collect();
            types.push(name);

            // 跳过边界声明
            while i < chars.len() && chars[i] != '>' {
                match chars[i] {
                    'L' => {
                        i += 1;
                        while i < chars.len() && chars[i] != ';' {
                            i += 1;
                        }
                        i += 1;
                    }
                    ':' => {
                        i += 1;
                    }
                    _ => {
                        i += 1;
                    }
                }
                // 如果遇到下一个类型变量 (非 : 开头) 就 break
                if i < chars.len() && chars[i].is_ascii_uppercase() && chars[i] != 'L' {
                    break;
                }
            }
        }
    }

    types
}

/// 从泛型签名中提取返回类型的实际类名
/// 正确处理多层嵌套泛型，如:
///   `LResult<LPageResponse<LUserDTO;>;>;` -> "UserDTO" (最内层)
///   `LResult<Ljava/lang/Void;>;` -> "Result" (回退到外层包装器)
///   `LResult<LUserDTO;>;` -> "UserDTO" (最内层)
/// 同时将包装器类型也加入候选，确保内层为 Void 时也能解析到包装器
pub fn extract_return_type_from_signature(signature: &str) -> Option<String> {
    let return_part = signature.split(')').nth(1)?;
    let chars: Vec<char> = return_part.chars().collect();

    let mut candidates: Vec<(usize, String)> = Vec::new();
    let mut i = 0;

    // 提取外层类型 (在 '<' 或 ';' 之前的 L... 类型)
    if i < chars.len() && chars[i] == 'L' {
        let start = i + 1;
        while i < chars.len() && chars[i] != '<' && chars[i] != ';' {
            i += 1;
        }
        let outer_class: String = chars[start..i].iter().collect();
        if !outer_class.starts_with("java/") && !outer_class.is_empty() {
            candidates.push((0, outer_class));
        }
    } else {
        // 跳过到 '<' 或 ';'
        while i < chars.len() && chars[i] != '<' && chars[i] != ';' {
            i += 1;
        }
    }

    // 递归收集泛型参数中的内层类名 (depth >= 1)
    if i < chars.len() && chars[i] == '<' {
        let mut depth = 1usize;
        collect_type_candidates(&chars, &mut i, &mut depth, &mut candidates);
    }

    // 优先返回最深层（最内层）的非 java 类型
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    candidates
        .first()
        .map(|(_, class_name)| class_name.replace('/', "."))
}

/// 递归收集签名中的非 java 类型候选
fn collect_type_candidates(
    chars: &[char],
    i: &mut usize,
    depth: &mut usize,
    candidates: &mut Vec<(usize, String)>,
) {
    while *i < chars.len() {
        match chars[*i] {
            'L' => {
                *i += 1;
                let start = *i;
                // 扫描类名，跳过嵌套的 <...>
                let mut nesting = 0;
                while *i < chars.len() {
                    match chars[*i] {
                        '<' => {
                            nesting += 1;
                            *i += 1;
                            // 递归处理泛型参数
                            collect_type_candidates(chars, i, &mut (*depth + nesting), candidates);
                        }
                        ';' => {
                            if nesting == 0 {
                                break;
                            }
                            nesting -= 1;
                            *i += 1;
                        }
                        _ => {
                            *i += 1;
                        }
                    }
                }
                let class_name: String = chars[start..*i].iter().collect();
                if !class_name.starts_with("java/") && !class_name.contains('<') {
                    candidates.push((*depth, class_name));
                }
                *i += 1; // skip ';'
            }
            '<' => {
                *i += 1;
                collect_type_candidates(chars, i, depth, candidates);
            }
            '>' => {
                *i += 1;
                return;
            }
            '*' | '+' | '-' => {
                // 通配符: * (=?), + (extends), - (super)
                *i += 1;
            }
            'T' => {
                // 类型变量: Tname;
                *i += 1;
                while *i < chars.len() && chars[*i] != ';' {
                    *i += 1;
                }
                *i += 1; // skip ';'
            }
            '[' => {
                *i += 1;
            }
            _ => {
                *i += 1;
            }
        }
    }
}

/// 递归提取 DTO 字段
pub fn extract_dto_fields(
    class_name: &str,
    parser: &JarParser,
    visited: &mut HashSet<String>,
) -> (Vec<ApiField>, Vec<NodeInfo>) {
    if visited.contains(class_name) {
        return (Vec::new(), Vec::new());
    }
    visited.insert(class_name.to_string());

    let data = match parser.get_class_data(class_name) {
        Some(d) => d,
        None => return (Vec::new(), Vec::new()),
    };

    let class_file = match cafebabe::parse_class(data) {
        Ok(cf) => cf,
        Err(_) => return (Vec::new(), Vec::new()),
    };

    let mut fields = Vec::new();
    let mut nodes = Vec::new();

    for field in &class_file.fields {
        let field_name = field.name.to_string();

        // 跳过静态字段和内部字段
        if field
            .access_flags
            .contains(cafebabe::FieldAccessFlags::STATIC)
            || field_name.contains('$')
        {
            continue;
        }

        let field_type_name = annotation::get_field_type_name(&field.descriptor);

        // 获取 @ApiModelProperty 信息
        let (comment, required, example_value) =
            annotation::get_api_model_property(&class_file, &field_name);

        // 尝试获取更精确的类型信息 (从 Signature 属性)
        let resolved_type = annotation::get_field_signature(&class_file, &field_name)
            .and_then(|sig| resolve_field_type_from_signature(&sig))
            .unwrap_or_else(|| field_type_name.clone());

        // 如果是自定义类型（非 Java 标准库），递归提取
        let is_custom_type =
            is_custom_type_private(&resolved_type) && parser.class_exists(&resolved_type);

        if is_custom_type {
            let (sub_fields, sub_nodes) = extract_dto_fields(&resolved_type, parser, visited);
            if !sub_fields.is_empty() {
                let short = resolved_type
                    .rsplit('.')
                    .next()
                    .unwrap_or(&resolved_type)
                    .to_string();
                nodes.push(NodeInfo {
                    node_name: short,
                    node_desc: comment.clone(),
                    resp_fields: sub_fields,
                });
                nodes.extend(sub_nodes);
            }
        }

        fields.push(ApiField {
            field_name,
            field_type: short_type_name(&resolved_type).to_string(),
            required,
            field_length: String::new(),
            comment,
            example_value,
        });
    }

    // 解析父类字段（处理继承）
    if let Some(parent_name) = resolve_super_class_name(&class_file) {
        if parent_name != "java/lang/Object" && parser.class_exists(&parent_name) {
            let parent_dot = parent_name.replace('/', ".");
            let (parent_fields, parent_nodes) = extract_dto_fields(&parent_dot, parser, visited);
            fields.extend(parent_fields);
            nodes.extend(parent_nodes);
        }
    }

    (fields, nodes)
}

/// 从 class 文件解析父类名称
fn resolve_super_class_name(class_file: &cafebabe::ClassFile) -> Option<String> {
    class_file
        .super_class
        .as_ref()
        .map(|cn| cn.deref().to_string())
}

/// 从字段签名中解析实际类型
fn resolve_field_type_from_signature(signature: &str) -> Option<String> {
    // 简单处理: 查找 L...; 中的非标准库类型
    let mut result = None;
    let mut i = 0;
    let chars: Vec<char> = signature.chars().collect();

    while i < chars.len() {
        if chars[i] == 'L' {
            let start = i + 1;
            let mut end = start;
            while end < chars.len() && chars[end] != ';' {
                end += 1;
            }
            let class_name: String = chars[start..end].iter().collect();
            if !class_name.starts_with("java/") {
                result = Some(simplify_class_name(&class_name));
                break;
            }
            i = end + 1;
        } else {
            i += 1;
        }
    }

    result
}

/// 非 Java 标准库且非数组的类型视为自定义类型
pub fn is_custom_type_private(type_name: &str) -> bool {
    !matches!(
        type_name,
        "String"
            | "Integer"
            | "Long"
            | "Double"
            | "Float"
            | "Boolean"
            | "Byte"
            | "Short"
            | "Character"
            | "byte"
            | "char"
            | "int"
            | "long"
            | "double"
            | "float"
            | "boolean"
            | "short"
            | "Date"
            | "LocalDateTime"
            | "LocalDate"
            | "BigDecimal"
            | "Object"
            | "List"
            | "Map"
            | "Set"
    ) && !type_name.contains("[]")
}

/// 从泛型签名中提取方法参数的实际类型列表
/// 对于包装器类型（如 HrmsAppRequest<XxxReq>），提取所有非 java 类型（含包装器和内层泛型）
/// 返回所有候选类型 internal name 列表（使用 / 分隔符）
pub fn extract_param_types_from_signature(signature: &str) -> Vec<String> {
    let chars: Vec<char> = signature.chars().collect();

    let mut i = 0;
    while i < chars.len() && chars[i] != '(' {
        i += 1;
    }
    if i >= chars.len() {
        return Vec::new();
    }
    i += 1; // skip '('

    let mut params = Vec::new();

    while i < chars.len() && chars[i] != ')' {
        match chars[i] {
            'L' => {
                let mut candidates: Vec<(usize, String)> = Vec::new();
                let mut depth = 0usize;
                parse_type_arg(&chars, &mut i, &mut depth, &mut candidates);

                // 返回所有非 java 候选类型（含包装器和内层类型）
                for (_, class_name) in &candidates {
                    params.push(class_name.clone());
                }
            }
            '[' => {
                while i < chars.len() && chars[i] == '[' {
                    i += 1;
                }
                if i < chars.len() && chars[i] == 'L' {
                    while i < chars.len() && chars[i] != ';' {
                        i += 1;
                    }
                    i += 1;
                } else if i < chars.len() {
                    i += 1;
                }
            }
            'T' => {
                while i < chars.len() && chars[i] != ';' {
                    i += 1;
                }
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    params
}

/// 解析单个类型参数 `L...<...>;`，收集非 java 的候选类型
fn parse_type_arg(
    chars: &[char],
    i: &mut usize,
    depth: &mut usize,
    candidates: &mut Vec<(usize, String)>,
) {
    // chars[*i] == 'L'
    *i += 1;
    let start = *i;
    let mut name_end = *i; // 纯类名（不含泛型）的结束位置
    let mut nesting = 0;

    while *i < chars.len() {
        match chars[*i] {
            '<' => {
                if nesting == 0 {
                    name_end = *i; // 类名在 < 之前结束
                }
                nesting += 1;
                *i += 1;
                let inner_depth = *depth + nesting;
                parse_generic_args(chars, i, inner_depth, candidates);
                nesting -= 1;
            }
            ';' => {
                if nesting == 0 {
                    if name_end == start {
                        name_end = *i; // 没有泛型，类名到 ; 结束
                    }
                    break;
                }
                *i += 1;
            }
            _ => {
                *i += 1;
            }
        }
    }

    let class_name: String = chars[start..name_end].iter().collect();
    if !class_name.starts_with("java/") && !class_name.is_empty() {
        candidates.push((*depth, class_name));
    }

    if *i < chars.len() && chars[*i] == ';' {
        *i += 1;
    }
}

/// 解析 `<...>` 泛型参数块中的所有类型
fn parse_generic_args(
    chars: &[char],
    i: &mut usize,
    depth: usize,
    candidates: &mut Vec<(usize, String)>,
) {
    while *i < chars.len() && chars[*i] != '>' {
        match chars[*i] {
            'L' => {
                let mut d = depth;
                parse_type_arg(chars, i, &mut d, candidates);
            }
            'T' => {
                *i += 1;
                while *i < chars.len() && chars[*i] != ';' {
                    *i += 1;
                }
                if *i < chars.len() {
                    *i += 1;
                }
            }
            '*' | '+' | '-' => {
                *i += 1;
            }
            '[' => {
                while *i < chars.len() && chars[*i] == '[' {
                    *i += 1;
                }
            }
            _ => {
                *i += 1;
            }
        }
    }

    // 跳过 >
    if *i < chars.len() && chars[*i] == '>' {
        *i += 1;
    }
}

/// 从方法描述符获取返回类型
pub fn get_return_type_from_descriptor(descriptor: &str) -> String {
    let after_paren = match descriptor.split(')').nth(1) {
        Some(s) => s,
        None => return "void".to_string(),
    };

    match after_paren {
        "V" => "void".to_string(),
        s if s.len() == 1 => resolve_base_type(s.chars().next().unwrap()).to_string(),
        s if s.starts_with('L') => {
            let class_name = s.trim_start_matches('L').trim_end_matches(';');
            class_name.replace('/', ".")
        }
        s if s.starts_with('[') => {
            let dims = s.chars().take_while(|c| *c == '[').count();
            let element = &s[dims..];
            let base = if element.starts_with('L') {
                let cn = element.trim_start_matches('L').trim_end_matches(';');
                return format!("{}{}", cn.replace('/', "."), "[]".repeat(dims));
            } else if element.len() == 1 {
                resolve_base_type(element.chars().next().unwrap())
            } else {
                "Object"
            };
            format!("{}{}", base, "[]".repeat(dims))
        }
        _ => "Object".to_string(),
    }
}
