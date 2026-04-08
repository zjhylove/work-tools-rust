use serde::{Deserialize, Serialize};

/// 列信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    /// 字段名
    pub name: String,
    /// 数据类型 (VARCHAR, INT...)
    pub data_type: String,
    /// 最大长度
    pub max_length: Option<u64>,
    /// 是否允许 NULL
    pub is_nullable: bool,
    /// 是否主键
    pub is_primary_key: bool,
    /// 默认值
    pub default_value: Option<String>,
    /// 字段注释
    pub comment: Option<String>,
    /// 列位置 (从 1 开始)
    pub position: u32,
}

impl ColumnInfo {
    /// 格式化数据类型 (如 VARCHAR(255), BIGINT)
    pub fn formatted_data_type(&self) -> String {
        if let Some(len) = self.max_length {
            format!("{}({})", self.data_type.to_uppercase(), len)
        } else {
            self.data_type.to_uppercase()
        }
    }

    /// 创建新的列信息
    pub fn new(name: impl Into<String>, data_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: data_type.into(),
            max_length: None,
            is_nullable: true,
            is_primary_key: false,
            default_value: None,
            comment: None,
            position: 0,
        }
    }
}
