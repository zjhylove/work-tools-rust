use serde::{Deserialize, Serialize};

/// 插件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDocConfig {
    /// JAR 文件路径
    #[serde(default)]
    pub source_jar_path: String,
    /// 服务名称
    #[serde(default)]
    pub service_name: String,
    /// 依赖 JAR 前缀列表
    #[serde(default)]
    pub dependency_jars: Vec<String>,
    /// 是否自动扫描依赖
    #[serde(default)]
    pub auto_scan_dependencies: bool,
}

/// 导出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Word,
    Markdown,
    Html,
}

impl Serialize for ExportFormat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            ExportFormat::Word => "word",
            ExportFormat::Markdown => "markdown",
            ExportFormat::Html => "html",
        })
    }
}

impl<'de> Deserialize<'de> for ExportFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "word" => Ok(ExportFormat::Word),
            "markdown" => Ok(ExportFormat::Markdown),
            "html" => Ok(ExportFormat::Html),
            _ => Ok(ExportFormat::Markdown),
        }
    }
}

impl ExportFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Word => "docx",
            ExportFormat::Markdown => "md",
            ExportFormat::Html => "html",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ExportFormat::Word => "Word",
            ExportFormat::Markdown => "Markdown",
            ExportFormat::Html => "HTML",
        }
    }
}

/// 导出配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// 选中的 API 列表
    pub selected_apis: Vec<SelectedApi>,
    /// 输出目录
    pub output_dir: String,
    /// 导出格式
    pub formats: Vec<ExportFormat>,
}

/// 选中的 API 标识
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedApi {
    /// 类名
    pub class_name: String,
    /// 方法名
    pub method_name: String,
}

/// 导出历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportHistory {
    /// 记录 ID
    pub id: String,
    /// 服务名称
    pub service_name: String,
    /// API 数量
    pub api_count: usize,
    /// 导出格式
    pub formats: Vec<ExportFormat>,
    /// 输出路径
    pub output_path: String,
    /// 导出时间 (ISO 8601)
    pub exported_at: String,
}
