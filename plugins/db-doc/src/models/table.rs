use serde::{Deserialize, Serialize};
use super::ColumnInfo;

/// 索引信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    /// 索引名
    pub name: String,
    /// 索引列
    pub columns: Vec<String>,
    /// 是否唯一索引
    pub is_unique: bool,
    /// 是否主键索引
    pub is_primary: bool,
}

impl IndexInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            columns: Vec::new(),
            is_unique: false,
            is_primary: false,
        }
    }
}

/// 表信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    /// 表名
    pub name: String,
    /// 所属 schema/数据库
    pub schema: String,
    /// 表注释
    pub comment: Option<String>,
    /// 所有列
    pub columns: Vec<ColumnInfo>,
    /// 索引信息
    pub indexes: Vec<IndexInfo>,
}

impl TableInfo {
    pub fn new(name: impl Into<String>, schema: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            schema: schema.into(),
            comment: None,
            columns: Vec::new(),
            indexes: Vec::new(),
        }
    }

    /// 获取主键列
    pub fn primary_key_columns(&self) -> Vec<&ColumnInfo> {
        self.columns
            .iter()
            .filter(|c| c.is_primary_key)
            .collect()
    }
}
