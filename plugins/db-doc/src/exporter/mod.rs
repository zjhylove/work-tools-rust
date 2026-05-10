mod html;
mod markdown;

pub use html::HtmlExporter;
pub use markdown::MarkdownExporter;

use crate::models::{ExportConfig, TableInfo};
use anyhow::Result;

/// 文档导出器 trait
pub trait DocumentExporter {
    /// 导出文档，返回生成的文件路径
    fn export(&self, tables: &[TableInfo], config: &ExportConfig) -> Result<Vec<String>>;
}
