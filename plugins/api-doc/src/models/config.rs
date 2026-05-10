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
}

/// 导出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Markdown,
    Html,
}

impl Serialize for ExportFormat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
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
            "markdown" => Ok(ExportFormat::Markdown),
            "html" => Ok(ExportFormat::Html),
            _ => Ok(ExportFormat::Markdown),
        }
    }
}

impl ExportFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Markdown => "md",
            ExportFormat::Html => "html",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
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
