use std::collections::HashSet;

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

/// 简化类名: java/lang/String -> String
pub fn simplify_class_name(internal_name: &str) -> String {
    match internal_name {
        "java/lang/String" => "String".to_string(),
        "java/lang/Integer" => "Integer".to_string(),
        "java/lang/Long" => "Long".to_string(),
        "java/lang/Double" => "Double".to_string(),
        "java/lang/Float" => "Float".to_string(),
        "java/lang/Boolean" => "Boolean".to_string(),
        "java/lang/Date" => "Date".to_string(),
        "java/time/LocalDateTime" => "LocalDateTime".to_string(),
        "java/time/LocalDate" => "LocalDate".to_string(),
        "java/math/BigDecimal" => "BigDecimal".to_string(),
        "java/util/List" => "List".to_string(),
        "java/util/Map" => "Map".to_string(),
        "java/util/Set" => "Set".to_string(),
        _ => internal_name
            .rsplit('/')
            .next()
            .unwrap_or(internal_name)
            .to_string(),
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
/// 例如 `Ljava/util/List<Lcom/example/dto/UserDTO;>;` -> "com/example/dto/UserDTO"
pub fn extract_return_type_from_signature(signature: &str) -> Option<String> {
    // 查找 ) 后的返回类型
    let return_part = signature.split(')').nth(1)?;

    // 提取最内层的 L...; 类型
    let mut depth = 0;
    let mut innermost_start = None;
    let mut innermost_end = None;

    for (i, c) in return_part.char_indices() {
        match c {
            '<' => {
                depth += 1;
                if depth == 1 {
                    innermost_start = None;
                }
            }
            '>' => {
                if depth == 1 && innermost_start.is_some() {
                    innermost_end = Some(i);
                }
                depth -= 1;
            }
            'L' => {
                if depth >= 1 {
                    innermost_start = Some(i + 1);
                }
            }
            ';' => {
                if depth >= 1 && innermost_start.is_some() && innermost_end.is_none() {
                    let class_name: String = return_part[innermost_start.unwrap()..i].to_string();
                    if !class_name.starts_with("java/") {
                        return Some(class_name.replace('/', "."));
                    }
                    innermost_start = None;
                }
            }
            _ => {}
        }
    }

    // 如果没有嵌套泛型，直接查找第一个 L...;
    if let Some(pos) = return_part.find('L') {
        if let Some(end) = return_part[pos..].find(';') {
            let class_name = &return_part[pos + 1..pos + end];
            if !class_name.starts_with("java/") {
                return Some(class_name.replace('/', "."));
            }
        }
    }

    None
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
        let field_type_name = annotation::get_field_type_name(&field.descriptor);

        // 跳过静态字段和内部字段
        if field_name.contains('$') {
            continue;
        }

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
                nodes.push(NodeInfo {
                    node_name: resolved_type.clone(),
                    node_desc: comment.clone(),
                    resp_fields: sub_fields,
                });
                nodes.extend(sub_nodes);
            }
        }

        fields.push(ApiField {
            field_name,
            field_type: resolved_type,
            required,
            field_length: String::new(),
            comment,
            example_value,
        });
    }

    (fields, nodes)
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
/// 对于包装器类型（如 HrmsAppRequest<PraiseV2Req>），提取内层泛型参数
/// 签名格式: `(Lcom/example/dto/Wrapper<Lcom/example/dto/UserDTO;>;)Lcom/example/vo/Result<Lcom/example/vo/UserVO;>;`
/// 返回参数的泛型内层类型 internal name 列表
pub fn extract_param_types_from_signature(signature: &str) -> Vec<String> {
    let chars: Vec<char> = signature.chars().collect();

    // 找到 ( 开始
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
                let start = i + 1;
                while i < chars.len() && chars[i] != ';' {
                    i += 1;
                }
                // i 现在在 ';' 位置
                // 检查 ';' 之后是否有 '<' (泛型参数)
                // 但实际上泛型签名格式是 Lcom/xxx/Foo<Lcom/xxx/Bar;>;
                // 即 < 在 ; 之前，不是之后
                // 需要重新扫描这个参数
                let class_path: String = chars[start..i].iter().collect();

                // 从 start 位置开始，找 <...> 泛型参数中的内层类型
                let mut j = start;
                let mut found_inner = false;
                while j < i {
                    if chars[j] == '<' {
                        // 找到了泛型参数，提取内层非 java 类型
                        if let Some(inner_type) = extract_innermost_type(&chars, j) {
                            if !inner_type.starts_with("java/") {
                                params.push(inner_type);
                                found_inner = true;
                            }
                        }
                        break;
                    }
                    j += 1;
                }
                if !found_inner && !class_path.starts_with("java/") {
                    // 没有泛型参数，直接用该类型
                    params.push(class_path);
                }

                i += 1; // skip ';'
                // 跳过多余的 > (泛型签名的闭合)
                while i < chars.len() && chars[i] == '>' {
                    i += 1;
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

/// 从 `<...>` 泛型参数中提取最内层的非 java/ 标准库类型
fn extract_innermost_type(chars: &[char], start: usize) -> Option<String> {
    let mut i = start;
    if i >= chars.len() || chars[i] != '<' {
        return None;
    }
    i += 1; // skip '<'

    let mut depth = 1;
    let mut last_non_java: Option<String> = None;

    while i < chars.len() && depth > 0 {
        match chars[i] {
            '<' => {
                depth += 1;
                i += 1;
            }
            '>' => {
                depth -= 1;
                i += 1;
            }
            'L' => {
                let start = i + 1;
                let mut end = start;
                while end < chars.len() && chars[end] != ';' {
                    end += 1;
                }
                let class_path: String = chars[start..end].iter().collect();
                if !class_path.starts_with("java/") {
                    last_non_java = Some(class_path);
                }
                i = end + 1; // skip ';'
            }
            _ => {
                i += 1;
            }
        }
    }

    last_non_java
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
            simplify_class_name(class_name)
        }
        s if s.starts_with('[') => {
            let dims = s.chars().take_while(|c| *c == '[').count();
            let element = &s[dims..];
            let base = if element.starts_with('L') {
                let cn = element.trim_start_matches('L').trim_end_matches(';');
                return format!("{}{}", simplify_class_name(cn), "[]".repeat(dims));
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
