use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::exporter::sanitize_filename;
use crate::exporter::DocumentExporter;
use crate::models::ApiInfo;

pub struct MarkdownExporter;

impl DocumentExporter for MarkdownExporter {
    fn export(
        &self,
        apis: &[ApiInfo],
        output_dir: &str,
        service_name: &str,
    ) -> Result<Vec<String>> {
        let dir = Path::new(output_dir);
        fs::create_dir_all(dir)?;

        let mut files = Vec::new();

        // 生成汇总文件
        let mut summary = format!(
            "# {} API 文档\n\n> 生成时间: {}\n\n",
            service_name,
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        );

        // 目录
        summary.push_str("## 目录\n\n");
        for (i, api) in apis.iter().enumerate() {
            summary.push_str(&format!(
                "{}. [{}](#{}) - `{}` `{}`\n",
                i + 1,
                api.api_name,
                anchor_name(&api.api_name),
                api.http_method,
                api.full_path
            ));
        }
        summary.push_str("\n---\n\n");

        // 每个 API 的详情
        for api in apis {
            summary.push_str(&format!("## {}\n\n", api.api_name));
            summary.push_str(&format!(
                "- **HTTP 方法**: {}\n- **路径**: `{}`\n- **服务**: {}\n",
                api.http_method, api.full_path, api.service_name
            ));
            if !api.version.is_empty() {
                summary.push_str(&format!("- **版本**: {}\n", api.version));
            }
            if !api.business_module.is_empty() {
                summary.push_str(&format!("- **模块**: {}\n", api.business_module));
            }
            summary.push('\n');

            // 请求参数
            if !api.req_fields.is_empty() {
                summary.push_str("### 请求参数\n\n");
                summary.push_str("| 字段名 | 类型 | 必填 | 注释 |\n");
                summary.push_str("|--------|------|------|------|\n");
                for field in &api.req_fields {
                    summary.push_str(&format!(
                        "| {} | {} | {} | {} |\n",
                        field.field_name, field.field_type, field.required, field.comment
                    ));
                }
                summary.push('\n');

                // 请求嵌套节点
                for node in &api.req_nodes {
                    if !node.node_desc.is_empty() {
                        summary
                            .push_str(&format!("#### {} ({})\n\n", node.node_name, node.node_desc));
                    } else {
                        summary.push_str(&format!("#### {}\n\n", node.node_name));
                    }
                    if !node.resp_fields.is_empty() {
                        summary.push_str("| 字段名 | 类型 | 必填 | 注释 |\n");
                        summary.push_str("|--------|------|------|------|\n");
                        for field in &node.resp_fields {
                            summary.push_str(&format!(
                                "| {} | {} | {} | {} |\n",
                                field.field_name, field.field_type, field.required, field.comment
                            ));
                        }
                        summary.push('\n');
                    }
                }

                if !api.req_example.is_empty() {
                    summary.push_str("### 请求示例\n\n```json\n");
                    summary.push_str(&api.req_example);
                    summary.push_str("\n```\n\n");
                }
            }

            // 响应参数
            if !api.resp_nodes.is_empty() {
                summary.push_str("### 响应参数\n\n");
                for node in &api.resp_nodes {
                    if !node.node_desc.is_empty() {
                        summary
                            .push_str(&format!("#### {} ({})\n\n", node.node_name, node.node_desc));
                    } else {
                        summary.push_str(&format!("#### {}\n\n", node.node_name));
                    }
                    if !node.resp_fields.is_empty() {
                        summary.push_str("| 字段名 | 类型 | 注释 |\n");
                        summary.push_str("|--------|------|------|\n");
                        for field in &node.resp_fields {
                            summary.push_str(&format!(
                                "| {} | {} | {} |\n",
                                field.field_name, field.field_type, field.comment
                            ));
                        }
                        summary.push('\n');
                    }
                }
            }

            if !api.resp_example.is_empty() {
                summary.push_str("### 响应示例\n\n```json\n");
                summary.push_str(&api.resp_example);
                summary.push_str("\n```\n\n");
            }

            summary.push_str("---\n\n");
        }

        let filename = format!("{}-api-doc.md", sanitize_filename(service_name));
        let filepath = dir.join(&filename);
        fs::write(&filepath, &summary)?;
        files.push(filepath.to_string_lossy().to_string());

        Ok(files)
    }
}

fn anchor_name(name: &str) -> String {
    name.to_lowercase()
        .replace(' ', "-")
        .replace(|c: char| !c.is_alphanumeric() && c != '-', "")
}
