use anyhow::Result;
use std::fs;
use std::path::Path;

use worktools_plugin_api::escape_xml;

use crate::exporter::DocumentExporter;
use crate::exporter::sanitize_filename;
use crate::models::ApiInfo;

pub struct HtmlExporter;

impl DocumentExporter for HtmlExporter {
    fn export(
        &self,
        apis: &[ApiInfo],
        output_dir: &str,
        service_name: &str,
    ) -> Result<Vec<String>> {
        let dir = Path::new(output_dir);
        fs::create_dir_all(dir)?;

        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n<html lang=\"zh-CN\">\n<head>\n");
        html.push_str("<meta charset=\"UTF-8\">\n");
        html.push_str(&format!(
            "<title>{} API 文档</title>\n",
            escape_xml(service_name)
        ));
        html.push_str("<style>\n");
        html.push_str(STYLES);
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");

        html.push_str(&format!(
            "<h1>{} API 文档</h1>\n<p class=\"meta\">生成时间: {}</p>\n",
            escape_xml(service_name),
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ));

        // 目录
        html.push_str("<div class=\"toc\"><h2>目录</h2><ul>\n");
        for (i, api) in apis.iter().enumerate() {
            html.push_str(&format!(
                "<li><a href=\"#api-{}\">{} - {} {}</a></li>\n",
                i,
                escape_xml(&api.api_name),
                api.http_method,
                escape_xml(&api.full_path)
            ));
        }
        html.push_str("</ul></div>\n");

        // API 详情
        for (i, api) in apis.iter().enumerate() {
            html.push_str(&format!("<div class=\"api-section\" id=\"api-{}\">\n", i));
            html.push_str(&format!(
                "<h2>{} <span class=\"method-badge method-{}\">{}</span></h2>\n",
                escape_xml(&api.api_name),
                api.http_method.to_lowercase(),
                api.http_method
            ));
            html.push_str(&format!(
                "<p><strong>路径:</strong> <code>{}</code></p>\n",
                escape_xml(&api.full_path)
            ));

            if !api.req_fields.is_empty() {
                html.push_str("<h3>请求参数</h3>\n<table><tr><th>字段名</th><th>类型</th><th>必填</th><th>注释</th></tr>\n");
                for field in &api.req_fields {
                    html.push_str(&format!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                        escape_xml(&field.field_name),
                        escape_xml(&field.field_type),
                        field.required,
                        escape_xml(&field.comment)
                    ));
                }
                html.push_str("</table>\n");

                if !api.req_example.is_empty() {
                    html.push_str("<h3>请求示例</h3>\n<pre><code class=\"json\">");
                    html.push_str(&escape_xml(&api.req_example));
                    html.push_str("</code></pre>\n");
                }
            }

            if !api.resp_nodes.is_empty() {
                html.push_str("<h3>响应参数</h3>\n");
                for node in &api.resp_nodes {
                    let title = if node.node_desc.is_empty() {
                        escape_xml(&node.node_name)
                    } else {
                        format!("{} ({})", escape_xml(&node.node_name), escape_xml(&node.node_desc))
                    };
                    html.push_str(&format!("<h4>{}</h4>\n", title));
                    if !node.resp_fields.is_empty() {
                        html.push_str(
                            "<table><tr><th>字段名</th><th>类型</th><th>注释</th></tr>\n",
                        );
                        for field in &node.resp_fields {
                            html.push_str(&format!(
                                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                                escape_xml(&field.field_name),
                                escape_xml(&field.field_type),
                                escape_xml(&field.comment)
                            ));
                        }
                        html.push_str("</table>\n");
                    }
                }
            }

            if !api.resp_example.is_empty() {
                html.push_str("<h3>响应示例</h3>\n<pre><code class=\"json\">");
                html.push_str(&escape_xml(&api.resp_example));
                html.push_str("</code></pre>\n");
            }

            html.push_str("</div>\n<hr>\n");
        }

        html.push_str("</body>\n</html>");

        let filename = format!("{}-api-doc.html", sanitize_filename(service_name));
        let filepath = dir.join(&filename);
        fs::write(&filepath, &html)?;
        Ok(vec![filepath.to_string_lossy().to_string()])
    }
}

const STYLES: &str = r#"
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 1200px; margin: 0 auto; padding: 20px; color: #333; }
h1 { color: #1a1a1a; border-bottom: 2px solid #8EAADB; padding-bottom: 10px; }
h2 { color: #2c5aa0; margin-top: 30px; }
h3 { color: #444; }
h4 { color: #666; }
.api-section { margin: 20px 0; padding: 15px; border: 1px solid #e0e0e0; border-radius: 8px; }
table { border-collapse: collapse; width: 100%; margin: 10px 0; }
th { background: #8EAADB; color: white; padding: 10px 12px; text-align: left; }
td { border: 1px solid #ddd; padding: 8px 12px; }
tr:nth-child(even) { background: #f9f9f9; }
code { background: #f4f4f4; padding: 2px 6px; border-radius: 3px; font-family: Consolas, 'Courier New', monospace; }
pre { background: #2d2d2d; color: #ccc; padding: 15px; border-radius: 6px; overflow-x: auto; }
pre code { background: none; color: inherit; }
.method-badge { display: inline-block; padding: 3px 8px; border-radius: 4px; color: white; font-size: 12px; font-weight: bold; }
.method-get { background: #61affe; }
.method-post { background: #49cc90; }
.method-put { background: #fca130; }
.method-delete { background: #f93e3e; }
.method-patch { background: #50e3c2; }
.toc { background: #f5f5f5; padding: 15px; border-radius: 8px; margin: 20px 0; }
.toc ul { list-style: decimal; }
.toc a { color: #2c5aa0; text-decoration: none; }
.toc a:hover { text-decoration: underline; }
.meta { color: #888; font-size: 14px; }
hr { border: none; border-top: 1px solid #e0e0e0; margin: 30px 0; }
"#;
