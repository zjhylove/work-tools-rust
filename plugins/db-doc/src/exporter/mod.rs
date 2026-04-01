mod markdown;

pub use markdown::MarkdownExporter;

use anyhow::Result;
use crate::models::{TableInfo, ExportConfig};

/// 文档导出器 trait
pub trait DocumentExporter {
    /// 导出文档，返回生成的文件路径
    fn export(&self, tables: &[TableInfo], config: &ExportConfig) -> Result<Vec<String>>;
}
