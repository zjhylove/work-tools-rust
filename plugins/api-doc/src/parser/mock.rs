use crate::models::{ApiField, NodeInfo};

/// 根据 Java 类型生成模拟值
fn mock_value_for_type(field_type: &str, example_value: &str) -> String {
    // 优先使用 example_value
    if !example_value.is_empty() {
        return format!("\"{}\"", example_value);
    }

    match field_type {
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
        t if t.ends_with("[]") => "[]".to_string(),
        _ => "{}".to_string(), // 对象类型返回空对象
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
            mock_value_for_type(&field.field_type, &field.example_value)
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
            mock_value_for_type(&field.field_type, &field.example_value)
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
    generate_node_mock(root, nodes, "")
}

/// 获取类名的简名 (最后一个 . 或 / 之后的部分)
fn short_name(name: &str) -> &str {
    name.rsplit(['.', '/']).next().unwrap_or(name)
}

fn generate_node_mock(node: &NodeInfo, all_nodes: &[NodeInfo], indent: &str) -> String {
    let inner_indent = format!("  {}", indent);
    let mut lines = Vec::new();
    lines.push("{".to_string());

    for (i, field) in node.resp_fields.iter().enumerate() {
        // 检查字段是否引用了另一个节点（按简名匹配）
        let field_short = short_name(&field.field_type);
        let child_node = all_nodes
            .iter()
            .find(|n| short_name(&n.node_name) == field_short);
        let value = if let Some(child) = child_node {
            generate_node_mock(child, all_nodes, &inner_indent)
        } else {
            mock_value_for_type(&field.field_type, &field.example_value)
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
