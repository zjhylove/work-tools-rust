use anyhow::Result;
use std::fs;
use std::path::Path;

use worktools_plugin_api::escape_xml;

use crate::exporter::DocumentExporter;
use crate::models::{ExportConfig, TableInfo};

pub struct HtmlExporter;

impl DocumentExporter for HtmlExporter {
    fn export(&self, tables: &[TableInfo], config: &ExportConfig) -> Result<Vec<String>> {
        let dir = Path::new(&config.output_dir);
        fs::create_dir_all(dir)?;

        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n<html lang=\"zh-CN\">\n<head>\n");
        html.push_str("<meta charset=\"UTF-8\">\n");
        html.push_str("<title>数据库文档</title>\n");
        html.push_str("<style>\n");
        html.push_str(STYLES);
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");

        html.push_str("<h1>数据库文档</h1>\n");
        html.push_str(&format!(
            "<p class=\"meta\">生成时间: {}</p>\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ));

        // 目录
        html.push_str("<div class=\"toc\"><h2>目录</h2><ul>\n");
        for table in tables {
            html.push_str(&format!(
                "<li><a href=\"#table-{}\">{}</a></li>\n",
                escape_xml(&table.name),
                escape_xml(&table.name)
            ));
        }
        html.push_str("</ul></div>\n");

        // 表详情
        for table in tables {
            html.push_str(&format!(
                "<div class=\"table-section\" id=\"table-{}\">\n",
                escape_xml(&table.name)
            ));
            html.push_str(&format!(
                "<h2>{}</h2>\n",
                escape_xml(&table.name)
            ));

            if let Some(ref comment) = table.comment {
                if !comment.is_empty() {
                    html.push_str(&format!(
                        "<p class=\"table-comment\"><strong>表注释:</strong> {}</p>\n",
                        escape_xml(comment)
                    ));
                }
            }
            html.push_str(&format!(
                "<p><strong>所属库:</strong> {}</p>\n",
                escape_xml(&table.schema)
            ));

            // 字段表格
            html.push_str("<h3>字段列表</h3>\n");
            html.push_str("<table><tr><th>字段名</th><th>类型</th><th>可空</th><th>主键</th><th>默认值</th><th>说明</th></tr>\n");
            for col in &table.columns {
                let nullable = if col.is_nullable { "是" } else { "否" };
                let pk = if col.is_primary_key { "是" } else { "否" };
                let default = col.default_value.as_deref().unwrap_or("-");
                let comment = col.comment.as_deref().unwrap_or("-");

                html.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                    escape_xml(&col.name),
                    escape_xml(&col.formatted_data_type()),
                    nullable,
                    pk,
                    escape_xml(default),
                    escape_xml(comment)
                ));
            }
            html.push_str("</table>\n");

            // 索引信息
            if !table.indexes.is_empty() {
                html.push_str("<h3>索引列表</h3>\n");
                html.push_str("<table><tr><th>索引名</th><th>列</th><th>唯一</th><th>类型</th></tr>\n");
                for idx in &table.indexes {
                    let unique = if idx.is_unique { "是" } else { "否" };
                    let idx_type = if idx.is_primary { "主键" } else { "普通" };
                    let columns = idx.columns.join(", ");

                    html.push_str(&format!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                        escape_xml(&idx.name),
                        escape_xml(&columns),
                        unique,
                        idx_type
                    ));
                }
                html.push_str("</table>\n");
            }

            html.push_str("</div>\n<hr>\n");
        }

        html.push_str("</body>\n</html>");

        let filename = format!(
            "数据库文档_{}.html",
            chrono::Local::now().format("%Y%m%d")
        );
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
.table-section { margin: 20px 0; padding: 15px; border: 1px solid #e0e0e0; border-radius: 8px; }
.table-comment { color: #666; font-size: 14px; }
table { border-collapse: collapse; width: 100%; margin: 10px 0; }
th { background: #8EAADB; color: white; padding: 10px 12px; text-align: left; }
td { border: 1px solid #ddd; padding: 8px 12px; }
tr:nth-child(even) { background: #f9f9f9; }
.toc { background: #f5f5f5; padding: 15px; border-radius: 8px; margin: 20px 0; }
.toc ul { list-style: decimal; }
.toc a { color: #2c5aa0; text-decoration: none; }
.toc a:hover { text-decoration: underline; }
.meta { color: #888; font-size: 14px; }
hr { border: none; border-top: 1px solid #e0e0e0; margin: 30px 0; }
"#;
