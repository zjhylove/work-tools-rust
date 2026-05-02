//! # 对象存储数据模型
//!
//! 定义对象存储插件使用的所有数据结构。
//!
//! ## Rust 知识点
//! - `Option<T>`: 可空类型，替代 null
//! - `Vec<T>`: 动态数组
//! - `#[serde(default)]`: 序列化时缺失字段使用默认值

use serde::{Deserialize, Serialize};

/// 云服务连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: String,         // UUID
    pub provider: String,   // "aliyun" 或 "tencent"
    pub name: String,       // 连接显示名称
    pub access_key: String, // 加密后的 Access Key
    pub secret_key: String, // 加密后的 Secret Key
    pub region: String,     // 区域（如 oss-cn-hangzhou）
    #[serde(default)] // 允许空字符串作为默认值
    pub bucket: String, // 默认存储桶名称
    pub endpoint: Option<String>, // 自定义 endpoint（用于私有云/专有云）
}

/// 存储桶信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketInfo {
    pub name: String,
    pub region: Option<String>,
    pub creation_date: Option<String>,
}

/// 对象（文件）信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInfo {
    pub key: String,           // 对象在存储桶中的路径
    pub size: u64,             // 文件大小（字节）
    pub last_modified: String, // 最后修改时间
    pub etag: String,          // ETag（用于校验和比较）
    pub is_dir: bool,          // 是否目录（以 / 结尾）
}
