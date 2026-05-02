//! # 对象存储统一抽象层
//!
//! 定义了统一的 trait，使得阿里云 OSS 和腾讯云 COS 可以通过相同的接口访问。
//!
//! ## 设计模式: 策略模式 (Strategy Pattern)
//! `ObjectStoreProvider` trait 定义了对象存储的通用接口。
//! `OssClient` 和 `CosClient` 各自实现这个 trait，提供厂商特定的逻辑。
//! 调用方只需要使用 `Box<dyn ObjectStoreProvider>`，不关心具体实现。
//!
//! ## Rust 知识点
//! - `trait`: 定义共享行为
//! - `Box<dyn Trait>`: 动态分发，运行时多态
//! - `Result<T>`: 使用 anyhow 的错误类型（灵活的错误处理）
//! - 关联常量/类型: trait 可以定义类型别名（如 `type HmacSha1`）

use crate::models::{BucketInfo, ObjectInfo};
use anyhow::Result;
use hmac::Hmac;
use sha1::Sha1;

/// HMAC-SHA1 的类型别名
/// 在多个云服务商的 API 签名中都会用到
pub type HmacSha1 = Hmac<Sha1>;

/// 云服务商统一接口
///
/// ## 为什么用 trait 而不是 enum？
/// - trait: 可以无限制地添加新的云服务商（开放封闭原则）
/// - enum: 添加新的服务商需要修改所有 match 分支
///
/// trait 更适合"需要扩展但不需要穷举"的场景。
pub trait ObjectStoreProvider {
    /// 列出所有存储桶（Bucket）
    fn list_buckets(&self, region: &str) -> Result<Vec<BucketInfo>>;

    /// 列出存储桶中的对象（支持前缀过滤、分隔符、最大数量限制）
    /// 返回 (对象列表, 公共前缀列表)
    fn list_objects(
        &self,
        bucket: &str,
        region: &str,
        prefix: &str,
        delimiter: Option<&str>,
        max_keys: Option<u32>,
    ) -> Result<(Vec<ObjectInfo>, Vec<String>)>;

    /// 下载对象内容（返回字节数组）
    fn get_object(&self, bucket: &str, region: &str, key: &str) -> Result<Vec<u8>>;

    /// 获取对象元数据（不下载内容）
    fn head_object(&self, bucket: &str, region: &str, key: &str) -> Result<ObjectInfo>;

    /// 上传对象
    fn put_object(
        &self,
        bucket: &str,
        region: &str,
        key: &str,
        data: &[u8],
        content_type: &str,
    ) -> Result<()>;

    /// 删除对象
    fn delete_object(&self, bucket: &str, region: &str, key: &str) -> Result<()>;
}

// ── 共享工具函数 ──

/// 移除 XML 标签（用于解析云服务商的 XML 响应）
/// 例如: strip_tag("<Bucket>my-bucket</Bucket>", "Bucket") → "my-bucket"
pub fn strip_tag(s: &str, tag: &str) -> String {
    s.trim()
        .replace(&format!("<{}>", tag), "")
        .replace(&format!("</{}>", tag), "")
        .trim()
        .to_string()
}

/// URL 编码（application/x-www-form-urlencoded 格式）
pub fn urlenc(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

/// 路径百分比编码（保持 `/` 不变，只编码路径中的特殊字符）
///
/// ## Rust 知识点: 迭代器链
/// `s.split('/').map(urlenc).collect::<Vec<_>>().join("/")`:
/// 1. 按 `/` 分割路径
/// 2. 对每个段进行 URL 编码
/// 3. 用 `/` 重新连接
/// 这样可以保持路径结构的同时编码特殊字符。
pub fn pct_encode(s: &str) -> String {
    s.split('/').map(urlenc).collect::<Vec<_>>().join("/")
}
