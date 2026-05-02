use crate::models::{ExportConfig, TableInfo, TemplateStyle};
use anyhow::Result;
use std::path::Path;

/// Markdown 文档导出器
pub struct MarkdownExporter {
    template_style: TemplateStyle,
}

impl MarkdownExporter {
    pub fn new(template_style: TemplateStyle) -> Self {
        Self { template_style }
    }

    /// 导出单张表到 Markdown 文件
    pub fn export_table(&self, table: &TableInfo, output_path: &Path) -> Result<()> {
        let content = self.render_table(table);
        std::fs::write(output_path, content)?;
        Ok(())
    }

    /// 导出多张表到一个 Markdown 文件
    pub fn export_tables(&self, tables: &[TableInfo], output_path: &Path) -> Result<()> {
        let mut content = String::new();
        content.push_str("# 数据库文档\n\n");
        content.push_str(&format!(
            "> 生成时间: {}\n\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
        ));

        // 目录
        content.push_str("## 目录\n\n");
        for table in tables {
            content.push_str(&format!(
                "- [{}](#{})\n",
                table.name,
                table.name.to_lowercase()
            ));
        }
        content.push_str("\n---\n\n");

        for table in tables {
            content.push_str(&self.render_table(table));
            content.push_str("\n---\n\n");
        }

        std::fs::write(output_path, content)?;
        Ok(())
    }

    /// 渲染单张表
    fn render_table(&self, table: &TableInfo) -> String {
        match self.template_style {
            TemplateStyle::Simple => self.render_simple(table),
            TemplateStyle::Detailed => self.render_detailed(table),
        }
    }

    /// 简洁模板
    fn render_simple(&self, table: &TableInfo) -> String {
        let mut md = String::new();

        md.push_str(&format!("## {}\n\n", table.name));

        if let Some(ref comment) = table.comment {
            if !comment.is_empty() {
                md.push_str(&format!("> {}\n\n", comment));
            }
        }

        // 表格头
        md.push_str("| 字段 | 类型 | 说明 |\n");
        md.push_str("|------|------|------|\n");

        for col in &table.columns {
            let comment = col.comment.as_deref().unwrap_or("-");
            md.push_str(&format!(
                "| {} | {} | {} |\n",
                col.name,
                col.formatted_data_type(),
                comment
            ));
        }

        md
    }

    /// 详细模板
    fn render_detailed(&self, table: &TableInfo) -> String {
        let mut md = String::new();

        md.push_str(&format!("## {}\n\n", table.name));

        // 表信息
        if let Some(ref comment) = table.comment {
            if !comment.is_empty() {
                md.push_str(&format!("**表注释**: {}\n\n", comment));
            }
        }

        md.push_str(&format!("**所属库**: {}\n\n", table.schema));

        // 字段表格
        md.push_str("### 字段列表\n\n");
        md.push_str("| 字段 | 类型 | 可空 | 主键 | 默认值 | 说明 |\n");
        md.push_str("|------|------|------|------|--------|------|\n");

        for col in &table.columns {
            let nullable = if col.is_nullable { "是" } else { "否" };
            let pk = if col.is_primary_key { "是" } else { "否" };
            let default = col.default_value.as_deref().unwrap_or("-");
            let comment = col.comment.as_deref().unwrap_or("-");

            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                col.name,
                col.formatted_data_type(),
                nullable,
                pk,
                default,
                comment
            ));
        }

        // 索引信息
        if !table.indexes.is_empty() {
            md.push_str("\n### 索引列表\n\n");
            md.push_str("| 索引名 | 列 | 唯一 | 类型 |\n");
            md.push_str("|--------|-----|------|------|\n");

            for idx in &table.indexes {
                let unique = if idx.is_unique { "是" } else { "否" };
                let idx_type = if idx.is_primary { "主键" } else { "普通" };
                let columns = idx.columns.join(", ");

                md.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    idx.name, columns, unique, idx_type
                ));
            }
        }

        md
    }
}

use super::DocumentExporter;

impl DocumentExporter for MarkdownExporter {
    fn export(&self, tables: &[TableInfo], config: &ExportConfig) -> Result<Vec<String>> {
        let output_path = std::path::PathBuf::from(&config.output_dir);
        std::fs::create_dir_all(&output_path)?;

        let file_path = output_path.join(format!(
            "数据库文档_{}.md",
            chrono::Local::now().format("%Y%m%d")
        ));
        self.export_tables(tables, &file_path)?;
        Ok(vec![file_path.to_string_lossy().to_string()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ColumnInfo, IndexInfo};

    fn create_test_table() -> TableInfo {
        let mut table = TableInfo::new("users", "mydb");
        table.comment = Some("用户表".to_string());
        table.columns = vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "bigint".to_string(),
                max_length: None,
                is_nullable: false,
                is_primary_key: true,
                default_value: None,
                comment: Some("用户ID".to_string()),
                position: 1,
            },
            ColumnInfo {
                name: "username".to_string(),
                data_type: "varchar".to_string(),
                max_length: Some(255),
                is_nullable: false,
                is_primary_key: false,
                default_value: None,
                comment: Some("用户名".to_string()),
                position: 2,
            },
        ];
        table.indexes = vec![IndexInfo {
            name: "idx_username".to_string(),
            columns: vec!["username".to_string()],
            is_unique: true,
            is_primary: false,
        }];
        table
    }

    #[test]
    fn test_render_simple() {
        let exporter = MarkdownExporter::new(TemplateStyle::Simple);
        let table = create_test_table();
        let md = exporter.render_table(&table);

        assert!(md.contains("## users"));
        assert!(md.contains("用户表"));
        assert!(md.contains("| id |"));
        assert!(md.contains("| username |"));
    }

    #[test]
    fn test_render_detailed() {
        let exporter = MarkdownExporter::new(TemplateStyle::Detailed);
        let table = create_test_table();
        let md = exporter.render_table(&table);

        assert!(md.contains("### 字段列表"));
        assert!(md.contains("### 索引列表"));
        assert!(md.contains("BIGINT"));
        assert!(md.contains("VARCHAR(255)"));
        assert!(md.contains("idx_username"));
    }
}
