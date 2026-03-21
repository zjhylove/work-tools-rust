use serde::{Deserialize, Serialize};

/// 数据库类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    MySQL,
    PostgreSQL,
}

impl DatabaseType {
    pub fn default_port(&self) -> u16 {
        match self {
            DatabaseType::MySQL => 3306,
            DatabaseType::PostgreSQL => 5432,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DatabaseType::MySQL => "mysql",
            DatabaseType::PostgreSQL => "postgresql",
        }
    }
}

impl std::fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// 配置 ID
    pub id: String,
    /// 配置名称 (如 "生产环境")
    pub name: String,
    /// 数据库类型
    pub db_type: DatabaseType,
    /// 主机地址
    pub host: String,
    /// 端口
    pub port: u16,
    /// 数据库名
    pub database: String,
    /// 用户名
    pub username: String,
    /// 密码 (加密存储)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// 创建时间 (Unix 时间戳)
    pub created_at: u64,
    /// 最后使用时间
    pub last_used: Option<u64>,
}

impl ConnectionConfig {
    pub fn new(name: impl Into<String>, db_type: DatabaseType) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            db_type,
            host: "localhost".to_string(),
            port: db_type.default_port(),
            database: String::new(),
            username: "root".to_string(),
            password: None,
            created_at: now,
            last_used: None,
        }
    }

    /// 构建 JDBC URL (用于 sqlx)
    pub fn to_connection_string(&self) -> String {
        match self.db_type {
            DatabaseType::MySQL => {
                format!(
                    "mysql://{}:{}@{}:{}/{}",
                    self.username,
                    self.password.as_deref().unwrap_or(""),
                    self.host,
                    self.port,
                    self.database
                )
            }
            DatabaseType::PostgreSQL => {
                format!(
                    "postgres://{}:{}@{}:{}/{}",
                    self.username,
                    self.password.as_deref().unwrap_or(""),
                    self.host,
                    self.port,
                    self.database
                )
            }
        }
    }
}

/// 导出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Word,
    Markdown,
    Pdf,
}

impl ExportFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Word => "docx",
            ExportFormat::Markdown => "md",
            ExportFormat::Pdf => "pdf",
        }
    }
}

/// 模板风格
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TemplateStyle {
    Simple,      // 简洁版
    Detailed,    // 详细版
    Enterprise,  // 企业版
}

impl Default for TemplateStyle {
    fn default() -> Self {
        Self::Detailed
    }
}

/// 导出配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// 连接配置 ID
    pub connection_id: String,
    /// 选中的表
    pub tables: Vec<String>,
    /// 输出目录
    pub output_dir: String,
    /// 导出格式
    pub format: ExportFormat,
    /// 模板风格
    pub template: TemplateStyle,
}

/// 导出历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportHistory {
    /// 记录 ID
    pub id: String,
    /// 连接配置名称
    pub connection_name: String,
    /// 导出的表
    pub tables: Vec<String>,
    /// 导出格式
    pub format: ExportFormat,
    /// 模板风格
    pub template: TemplateStyle,
    /// 输出路径
    pub output_path: String,
    /// 导出时间 (ISO 8601)
    pub exported_at: String,
}
