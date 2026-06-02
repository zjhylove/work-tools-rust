use crate::models::{ApiField, NodeInfo};

/// 根据 Java 类型生成模拟值
/// `item_indent` — 用于多行值（如数组）的内容缩进级别
fn mock_value_for_type(field: &ApiField, all_nodes: &[NodeInfo], item_indent: &str) -> String {
    // 优先使用 example_value
    if !field.example_value.is_empty() {
        return format!("\"{}\"", field.example_value);
    }

    // 如果是集合类型，生成数组
    if let Some(ref info) = field.collection_info {
        return mock_collection_value(info, &field.example_value, all_nodes, item_indent);
    }

    match field.field_type.as_str() {
        t if t.ends_with("[]") => "[]".to_string(),
        _ => mock_literal(field.field_type.as_str(), "{}"),
    }
}

/// 为集合类型生成 Mock 值
/// `item_indent` — 数组中每个元素的缩进级别
fn mock_collection_value(
    info: &crate::models::CollectionInfo,
    example_value: &str,
    all_nodes: &[NodeInfo],
    item_indent: &str,
) -> String {
    // 如果有 example_value，包装成单元素数组
    if !example_value.is_empty() {
        return format!("[\"{}\"]", example_value);
    }

    // 提取元素类型的短名称（处理可能包含完整包名的情况）
    let element_short = if info.element_type.contains('.') {
        info.element_type
            .rsplit('.')
            .next()
            .unwrap_or(&info.element_type)
    } else {
        &info.element_type
    };

    // 检查元素类型是否匹配某个嵌套节点
    let child_node = all_nodes.iter().find(|n| {
        let node_short = short_name(&n.node_name);
        node_short == element_short ||
        n.node_name.ends_with(&format!(".{}", element_short)) ||
        n.node_name.ends_with(&format!("/{}", element_short)) ||
        (info.element_type.contains('.') && short_name(&info.element_type) == node_short)
    });

    if let Some(child) = child_node {
        // 递归生成嵌套对象的 Mock（多元素数组，展示 2 个示例）
        // 块级模式：数组元素需要左花括号有独立缩进
        let obj_value1 = generate_node_mock(child, all_nodes, item_indent, true);
        let obj_value2 = generate_node_mock(child, all_nodes, item_indent, true);

        format!(
            "[\n{},\n{}\n{}]",
            obj_value1,
            obj_value2,
            item_indent
        )
    } else {
        // 基础类型值（单元素数组）
        format!("[{}]", mock_literal(&info.element_type, "\"\""))
    }
}

/// 生成基本类型的 mock JSON 字面量值
fn mock_literal(type_name: &str, fallback: &str) -> String {
    match type_name {
        "String" => "\"string\"".to_string(),
        "Integer" | "int" => "0".to_string(),
        "Long" | "long" => "0".to_string(),
        "Double" | "double" => "0.0".to_string(),
        "Float" | "float" => "0.0".to_string(),
        "Boolean" | "boolean" => "true".to_string(),
        "Byte" | "byte" => "0".to_string(),
        "Short" | "short" => "0".to_string(),
        "Character" | "char" => "\"a\"".to_string(),
        "Date" => "\"2024-01-01\"".to_string(),
        "LocalDateTime" => "\"2024-01-01T00:00:00\"".to_string(),
        "LocalDate" => "\"2024-01-01\"".to_string(),
        "BigDecimal" => "\"0.00\"".to_string(),
        _ => fallback.to_string(),
    }
}

/// 生成请求参数的 mock JSON（支持嵌套节点）
pub fn generate_req_mock_json(fields: &[ApiField], nodes: &[NodeInfo]) -> String {
    if fields.is_empty() {
        return "{}".to_string();
    }

    let mut lines = Vec::new();
    lines.push("{".to_string());

    for (i, field) in fields.iter().enumerate() {
        let comma = if i < fields.len() - 1 { "," } else { "" };

        // 检查字段类型是否匹配某个嵌套节点
        let field_short = short_name(&field.field_type);
        let child_node = nodes
            .iter()
            .find(|n| short_name(&n.node_name) == field_short);

        let value = if let Some(child) = child_node {
            generate_node_mock_inner(child, nodes, "  ")
        } else {
            mock_value_for_type(field, nodes, "    ")
        };

        lines.push(format!("  \"{}\": {}{}", field.field_name, value, comma));
    }

    lines.push("}".to_string());
    lines.join("\n")
}

fn generate_node_mock_inner(node: &NodeInfo, all_nodes: &[NodeInfo], base_indent: &str) -> String {
    let inner_indent = format!("  {}", base_indent);
    let mut lines = Vec::new();
    lines.push("{".to_string());

    for (i, field) in node.resp_fields.iter().enumerate() {
        let field_short = short_name(&field.field_type);
        let child_node = all_nodes
            .iter()
            .find(|n| short_name(&n.node_name) == field_short);
        let value = if let Some(child) = child_node {
            generate_node_mock_inner(child, all_nodes, &inner_indent)
        } else {
            mock_value_for_type(field, all_nodes, &inner_indent)
        };

        let comma = if i < node.resp_fields.len() - 1 {
            ","
        } else {
            ""
        };
        lines.push(format!(
            "{}\"{}\": {}{}",
            inner_indent, field.field_name, value, comma
        ));
    }

    lines.push(format!("{}}}", base_indent));
    lines.join("\n")
}

/// 生成响应的 mock JSON (从 resp_nodes 结构生成)
/// resp_nodes[0] 通常是外层 DTO (如 Result)，后续是嵌套 DTO (如 UserVO)
pub fn generate_resp_mock_json(nodes: &[NodeInfo]) -> String {
    if nodes.is_empty() {
        return "{}".to_string();
    }

    // 找到顶层节点：第一个 node 的字段中引用的类型在后续 nodes 中
    // 直接用第一个 node 作为根
    let root = &nodes[0];
    generate_node_mock(root, nodes, "", false)
}

/// 获取类名的简名 (最后一个 . 或 / 之后的部分)
fn short_name(name: &str) -> &str {
    name.rsplit(['.', '/']).next().unwrap_or(name)
}

/// 生成节点的 mock JSON
///
/// `block_mode`: false = 内联模式，左花括号紧跟 ": " 不含缩进（用于对象字段值）;
///               true  = 块级模式，左花括号有独立缩进（用于数组元素等多行上下文）
fn generate_node_mock(node: &NodeInfo, all_nodes: &[NodeInfo], indent: &str, block_mode: bool) -> String {
    let inner_indent = format!("  {}", indent);
    let mut lines = Vec::new();
    lines.push(if block_mode {
        format!("{}{{", indent)
    } else {
        "{".to_string()
    });

    for (i, field) in node.resp_fields.iter().enumerate() {
        let field_short = short_name(&field.field_type);
        let child_node = all_nodes
            .iter()
            .find(|n| short_name(&n.node_name) == field_short);
        let value = if let Some(child) = child_node {
            generate_node_mock(child, all_nodes, &inner_indent, false)
        } else {
            mock_value_for_type(field, all_nodes, &inner_indent)
        };

        let comma = if i < node.resp_fields.len() - 1 {
            ","
        } else {
            ""
        };
        lines.push(format!(
            "{}\"{}\": {}{}",
            inner_indent, field.field_name, value, comma
        ));
    }

    lines.push(format!("{}}}", indent));
    lines.join("\n")
}
