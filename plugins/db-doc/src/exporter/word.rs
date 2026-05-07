use anyhow::Result;
use std::io::{Cursor, Write};
use worktools_plugin_api::escape_xml;

use crate::models::{ExportConfig, TableInfo, TemplateStyle};

use super::DocumentExporter;

/// Word (DOCX) 文档导出器
pub struct WordExporter {
    template_style: TemplateStyle,
}

impl WordExporter {
    pub fn new(template_style: TemplateStyle) -> Self {
        Self { template_style }
    }

    // ---- OOXML package parts ----

    /// [Content_Types].xml
    fn build_content_types() -> String {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
</Types>"#
        .to_string()
    }

    /// _rels/.rels
    fn build_rels() -> String {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#
        .to_string()
    }

    /// word/_rels/document.xml.rels
    fn build_doc_rels() -> String {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#
        .to_string()
    }

    /// word/styles.xml
    fn build_styles() -> String {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="heading 1"/>
    <w:pPr><w:jc w:val="center"/></w:pPr>
    <w:rPr><w:b/><w:sz w:val="44"/><w:szCs w:val="44"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading2">
    <w:name w:val="heading 2"/>
    <w:rPr><w:b/><w:sz w:val="32"/><w:szCs w:val="32"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading3">
    <w:name w:val="heading 3"/>
    <w:rPr><w:b/><w:sz w:val="28"/><w:szCs w:val="28"/></w:rPr>
  </w:style>
</w:styles>"#
            .to_string()
    }

    // ---- Document body helpers ----

    /// Build a paragraph with optional style
    fn build_paragraph(text: &str, style: Option<&str>) -> String {
        let escaped = escape_xml(text);
        let ppr = match style {
            Some(s) => format!("<w:pPr><w:pStyle w:val=\"{}\"/></w:pPr>", s),
            None => String::new(),
        };
        format!("<w:p>{}<w:r><w:rPr><w:sz w:val=\"22\"/><w:szCs w:val=\"22\"/></w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>", ppr, escaped)
    }

    /// Build a bold paragraph (used for section labels)
    fn build_bold_paragraph(text: &str) -> String {
        let escaped = escape_xml(text);
        format!(
            "<w:p><w:pPr/><w:r><w:rPr><w:b/><w:sz w:val=\"22\"/><w:szCs w:val=\"22\"/></w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
            escaped
        )
    }

    /// Build a table cell with text
    fn build_cell(text: &str) -> String {
        let escaped = escape_xml(text);
        format!(
            "<w:tc><w:tcPr><w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"auto\"/></w:tcPr><w:p><w:r><w:rPr><w:sz w:val=\"20\"/><w:szCs w:val=\"20\"/></w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p></w:tc>",
            escaped
        )
    }

    /// Build a header cell (bold, gray background)
    fn build_header_cell(text: &str) -> String {
        let escaped = escape_xml(text);
        format!(
            "<w:tc><w:tcPr><w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"E0E0E0\"/></w:tcPr><w:p><w:pPr><w:jc w:val=\"center\"/></w:pPr><w:r><w:rPr><w:b/><w:sz w:val=\"20\"/><w:szCs w:val=\"20\"/></w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p></w:tc>",
            escaped
        )
    }

    /// Build a table row from a list of cell strings
    fn build_row(cells: &[String]) -> String {
        let inner: String = cells.join("");
        format!("<w:tr>{}</w:tr>", inner)
    }

    /// Open a table with full-width property
    fn open_table() -> &'static str {
        "<w:tbl><w:tblPr><w:tblW w:w=\"5000\" w:type=\"pct\"/><w:tblBorders>\
         <w:top w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"999999\"/>\
         <w:left w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"999999\"/>\
         <w:bottom w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"999999\"/>\
         <w:right w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"999999\"/>\
         <w:insideH w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"999999\"/>\
         <w:insideV w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"999999\"/>\
         </w:tblBorders></w:tblPr>"
    }

    // ---- Table rendering ----

    /// Render a simple template table (3 columns: 字段名, 类型, 说明)
    fn render_simple_table(&self, table: &TableInfo) -> String {
        let mut body = String::new();

        // Table heading
        let heading = match &table.comment {
            Some(c) if !c.is_empty() => format!("{} ({})", table.name, c),
            _ => table.name.clone(),
        };
        body.push_str(&Self::build_paragraph(&heading, Some("Heading2")));

        // Header row
        let headers = vec![
            Self::build_header_cell("字段名"),
            Self::build_header_cell("类型"),
            Self::build_header_cell("说明"),
        ];
        body.push_str(Self::open_table());
        body.push_str(&Self::build_row(&headers));

        // Data rows
        for col in &table.columns {
            let comment = col.comment.as_deref().unwrap_or("-");
            let cells = vec![
                Self::build_cell(&col.name),
                Self::build_cell(&col.formatted_data_type()),
                Self::build_cell(comment),
            ];
            body.push_str(&Self::build_row(&cells));
        }

        body.push_str("</w:tbl>");
        body.push_str("<w:p><w:r><w:t> </w:t></w:r></w:p>"); // spacing paragraph
        body
    }

    /// Render a detailed template table (6 columns) + index section
    fn render_detailed_table(&self, table: &TableInfo) -> String {
        let mut body = String::new();

        // Table heading
        body.push_str(&Self::build_paragraph(&table.name, Some("Heading2")));

        // Schema info
        body.push_str(&Self::build_bold_paragraph(&format!(
            "所属库: {}",
            table.schema
        )));
        if let Some(ref comment) = table.comment {
            if !comment.is_empty() {
                body.push_str(&Self::build_bold_paragraph(&format!("表注释: {}", comment)));
            }
        }

        // Field list heading
        body.push_str(&Self::build_paragraph("字段列表", Some("Heading3")));

        // Header row
        let headers = vec![
            Self::build_header_cell("字段名"),
            Self::build_header_cell("类型"),
            Self::build_header_cell("可空"),
            Self::build_header_cell("主键"),
            Self::build_header_cell("默认值"),
            Self::build_header_cell("说明"),
        ];
        body.push_str(Self::open_table());
        body.push_str(&Self::build_row(&headers));

        // Data rows
        for col in &table.columns {
            let nullable = if col.is_nullable { "是" } else { "否" };
            let pk = if col.is_primary_key { "是" } else { "否" };
            let default = col.default_value.as_deref().unwrap_or("-");
            let comment = col.comment.as_deref().unwrap_or("-");

            let cells = vec![
                Self::build_cell(&col.name),
                Self::build_cell(&col.formatted_data_type()),
                Self::build_cell(nullable),
                Self::build_cell(pk),
                Self::build_cell(default),
                Self::build_cell(comment),
            ];
            body.push_str(&Self::build_row(&cells));
        }
        body.push_str("</w:tbl>");

        // Index section
        if !table.indexes.is_empty() {
            body.push_str(&Self::build_paragraph("索引列表", Some("Heading3")));

            let idx_headers = vec![
                Self::build_header_cell("索引名"),
                Self::build_header_cell("列"),
                Self::build_header_cell("唯一"),
                Self::build_header_cell("类型"),
            ];
            body.push_str(Self::open_table());
            body.push_str(&Self::build_row(&idx_headers));

            for idx in &table.indexes {
                let unique = if idx.is_unique { "是" } else { "否" };
                let idx_type = if idx.is_primary { "主键" } else { "普通" };
                let columns = idx.columns.join(", ");

                let cells = vec![
                    Self::build_cell(&idx.name),
                    Self::build_cell(&columns),
                    Self::build_cell(unique),
                    Self::build_cell(idx_type),
                ];
                body.push_str(&Self::build_row(&cells));
            }
            body.push_str("</w:tbl>");
        }

        body.push_str("<w:p><w:r><w:t> </w:t></w:r></w:p>"); // spacing paragraph
        body
    }

    /// Build the complete document.xml for all tables
    fn build_document(&self, tables: &[TableInfo]) -> String {
        let mut body = String::new();

        // Title
        body.push_str(&Self::build_paragraph("数据库文档", Some("Heading1")));

        // Generation time
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        body.push_str(&Self::build_paragraph(
            &format!("生成时间: {}", timestamp),
            None,
        ));
        body.push_str("<w:p><w:r><w:t> </w:t></w:r></w:p>"); // spacing

        // Table of contents (plain text)
        body.push_str(&Self::build_paragraph("目录", Some("Heading2")));
        for table in tables {
            body.push_str(&Self::build_paragraph(&format!("- {}", table.name), None));
        }
        body.push_str("<w:p><w:r><w:t> </w:t></w:r></w:p>"); // spacing

        // Render each table
        for table in tables {
            match self.template_style {
                TemplateStyle::Simple => body.push_str(&self.render_simple_table(table)),
                TemplateStyle::Detailed => body.push_str(&self.render_detailed_table(table)),
            }
        }

        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    {}
  </w:body>
</w:document>"#,
            body
        )
    }

    /// Generate the DOCX bytes
    fn generate_docx(&self, tables: &[TableInfo]) -> Result<Vec<u8>> {
        let buf = Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(buf);
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // [Content_Types].xml
        zip.start_file("[Content_Types].xml", options)?;
        zip.write_all(Self::build_content_types().as_bytes())?;

        // _rels/.rels
        zip.start_file("_rels/.rels", options)?;
        zip.write_all(Self::build_rels().as_bytes())?;

        // word/document.xml
        zip.start_file("word/document.xml", options)?;
        zip.write_all(self.build_document(tables).as_bytes())?;

        // word/_rels/document.xml.rels
        zip.start_file("word/_rels/document.xml.rels", options)?;
        zip.write_all(Self::build_doc_rels().as_bytes())?;

        // word/styles.xml
        zip.start_file("word/styles.xml", options)?;
        zip.write_all(Self::build_styles().as_bytes())?;

        let buf = zip.finish()?;
        Ok(buf.into_inner())
    }
}

impl DocumentExporter for WordExporter {
    fn export(&self, tables: &[TableInfo], config: &ExportConfig) -> Result<Vec<String>> {
        let output_dir = std::path::PathBuf::from(&config.output_dir);
        std::fs::create_dir_all(&output_dir)?;

        let file_name = format!("数据库文档_{}.docx", chrono::Local::now().format("%Y%m%d"));
        let file_path = output_dir.join(&file_name);

        let docx_bytes = self.generate_docx(tables)?;
        std::fs::write(&file_path, docx_bytes)?;

        Ok(vec![file_path.to_string_lossy().to_string()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exporter::DocumentExporter;
    use crate::models::{ColumnInfo, IndexInfo};
    use std::io::Read;

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

    fn create_export_config(output_dir: &str) -> ExportConfig {
        ExportConfig {
            connection_id: "test-conn".to_string(),
            tables: vec!["users".to_string()],
            output_dir: output_dir.to_string(),
            format: crate::models::ExportFormat::Word,
            template: TemplateStyle::Detailed,
        }
    }

    #[test]
    fn test_build_content_types() {
        let xml = WordExporter::build_content_types();
        assert!(xml.contains("document.xml"));
        assert!(xml.contains("styles.xml"));
    }

    #[test]
    fn test_build_rels() {
        let xml = WordExporter::build_rels();
        assert!(xml.contains("word/document.xml"));
    }

    #[test]
    fn test_build_document_simple() {
        let exporter = WordExporter::new(TemplateStyle::Simple);
        let table = create_test_table();
        let doc = exporter.build_document(&[table]);

        assert!(doc.contains("users"));
        assert!(doc.contains("字段名"));
        assert!(doc.contains("类型"));
        assert!(doc.contains("说明"));
        assert!(doc.contains("BIGINT"));
        assert!(doc.contains("VARCHAR(255)"));
        assert!(doc.contains("用户ID"));
        assert!(doc.contains("用户名"));
    }

    #[test]
    fn test_build_document_detailed() {
        let exporter = WordExporter::new(TemplateStyle::Detailed);
        let table = create_test_table();
        let doc = exporter.build_document(&[table]);

        assert!(doc.contains("users"));
        assert!(doc.contains("字段名"));
        assert!(doc.contains("可空"));
        assert!(doc.contains("主键"));
        assert!(doc.contains("默认值"));
        assert!(doc.contains("索引列表"));
        assert!(doc.contains("idx_username"));
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(escape_xml("a<b>c&d"), "a&lt;b&gt;c&amp;d");
        assert_eq!(
            escape_xml("test\"value\""),
            "test&quot;value&quot;"
        );
    }

    #[test]
    fn test_generate_docx_is_valid_zip() {
        let exporter = WordExporter::new(TemplateStyle::Detailed);
        let table = create_test_table();
        let bytes = exporter.generate_docx(&[table]).unwrap();

        // Verify it is a valid ZIP
        let reader = zip::ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap();
        let names: Vec<&str> = reader.file_names().collect();
        assert!(names.contains(&"[Content_Types].xml"));
        assert!(names.contains(&"_rels/.rels"));
        assert!(names.contains(&"word/document.xml"));
        assert!(names.contains(&"word/_rels/document.xml.rels"));
        assert!(names.contains(&"word/styles.xml"));
    }

    #[test]
    fn test_generate_docx_document_content() {
        let exporter = WordExporter::new(TemplateStyle::Detailed);
        let table = create_test_table();
        let bytes = exporter.generate_docx(&[table]).unwrap();

        let mut archive = zip::ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap();
        let mut doc_file = archive.by_name("word/document.xml").unwrap();
        let mut content = String::new();
        doc_file.read_to_string(&mut content).unwrap();

        assert!(content.contains("users"));
        assert!(content.contains("用户ID"));
        assert!(content.contains("username"));
        assert!(content.contains("idx_username"));
    }

    #[test]
    fn test_export_creates_file() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let tmp_path = tmp_dir.path().to_string_lossy().to_string();

        let exporter = WordExporter::new(TemplateStyle::Simple);
        let table = create_test_table();
        let config = create_export_config(&tmp_path);

        let files = exporter.export(&[table], &config).unwrap();

        assert_eq!(files.len(), 1);
        let file_path = &files[0];
        assert!(file_path.ends_with(".docx"));

        // Verify the file exists
        assert!(std::path::Path::new(file_path).exists());

        // Verify it is a valid ZIP
        let bytes = std::fs::read(file_path).unwrap();
        zip::ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap();
    }
}
