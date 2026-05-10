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
    #[serde(default)]
    pub id: String,
    /// 配置名称 (如 "生产环境")
    pub name: String,
    /// 数据库类型
    #[serde(default)]
    pub db_type: DatabaseType,
    /// 主机地址
    #[serde(default = "default_host")]
    pub host: String,
    /// 端口
    #[serde(default)]
    pub port: u16,
    /// 数据库名
    #[serde(default)]
    pub database: String,
    /// 用户名
    #[serde(default = "default_username")]
    pub username: String,
    /// 密码 (加密存储)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// 创建时间 (Unix 时间戳)
    #[serde(default)]
    pub created_at: u64,
    /// 最后使用时间
    pub last_used: Option<u64>,
}

fn default_host() -> String {
    "localhost".to_string()
}

fn default_username() -> String {
    "root".to_string()
}

impl Default for DatabaseType {
    fn default() -> Self {
        DatabaseType::MySQL
    }
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
}

/// 导出配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// 连接配置 ID
    pub connection_id: String,
    /// 连接配置名称（导出时传入，用于文件名）
    #[serde(default)]
    pub connection_name: String,
    /// 选中的表
    pub tables: Vec<String>,
    /// 输出目录
    pub output_dir: String,
    /// 导出格式
    pub format: ExportFormat,
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
    /// 输出路径
    pub output_path: String,
    /// 导出时间 (ISO 8601)
    pub exported_at: String,
}
