//! # 数据库路由模型
//!
//! 定义数据库路由系统的数据结构。
//!
//! ## Rust 知识点
//! - `serde` 属性: 控制序列化行为
//! - `#[serde(default)]`: JSON 中缺少字段时使用 Default 值
//! - `Vec<String>`: 动态数组（可以增长和缩小）

use serde::{Deserialize, Serialize};

/// 路由规则 — 根据编号解析数据库和表的规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    pub id: String,              // 规则 ID（UUID）
    pub name: String,            // 规则名称
    pub description: String,     // 规则描述
    pub code_length: u32,        // 编号长度限制（0 表示不限制）
    pub code_prefix: String,     // 编号前缀过滤（逗号分隔，空表示不限制）
    pub route_script: String,    // Rhai 路由脚本（核心逻辑）
    #[serde(default)]            // 如果 JSON 中缺少此字段，使用 Vec::new()
    pub tables: Vec<String>,     // 关联的表名列表
}

/// 路由解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResult {
    pub database: String,        // 解析出的数据库名
    pub tables: Vec<String>,     // 解析出的表名列表
    pub code: String,            // 输入的编号
    pub rule_name: String,       // 使用的规则名称
    pub parse_time: String,      // 解析时间
}

/// 路由数据（顶层存储结构）
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RouteData {
    #[serde(default)]
    pub rules: Vec<RouteRule>,   // 所有的路由规则
}
