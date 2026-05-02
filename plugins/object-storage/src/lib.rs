//! # 对象存储插件
//!
//! 统一的云对象存储管理，支持阿里云 OSS 和腾讯云 COS。
//! 提供文件浏览、上传、下载、搜索、删除等功能。
//!
//! ## 架构设计
//! ```
//! ObjectStoragePlugin
//!   ├── ObjectStoreProvider (trait) ← 统一接口
//!   │   ├── OssClient ← 阿里云 OSS 实现
//!   │   └── CosClient ← 腾讯云 COS 实现
//!   ├── crypto ← 密钥加密存储
//!   └── models ← 数据模型
//! ```
//!
//! ## 设计模式: 策略模式 + 工厂方法
//! `build_provider()` 根据配置中的 provider 字段创建对应的客户端。
//! 新增云服务商只需：实现 ObjectStoreProvider → 在 match 中添加分支。
//!
//! ## Rust 知识点
//! - `Box<dyn ObjectStoreProvider>`: trait 对象，运行时多态
//! - `Mutex<PluginData>`: 线程安全的数据访问
//! - `match conn.provider.as_str()`: 字符串匹配分发
//! - `fn clean_endpoint`: 清理 URL 输入的工具函数

pub mod cos;
pub mod crypto;
pub mod models;
pub mod oss;
pub mod provider;

use anyhow::Context;
use models::ConnectionConfig;
use provider::ObjectStoreProvider;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Mutex;
use worktools_plugin_api::Plugin;

use cos::CosClient;
use oss::OssClient;

/// 插件持久化数据结构
#[derive(Debug, Default, Serialize, Deserialize)]
struct PluginData {
    connections: Vec<ConnectionConfig>,
}

/// 对象存储插件
///
/// 使用 `Mutex<PluginData>` 管理内部状态：
/// - `init()` 时从文件加载数据
/// - 每次操作时可能修改数据
/// - `handle_call` 需要 `&mut self`
pub struct ObjectStoragePlugin {
    data: Mutex<PluginData>,
}

impl ObjectStoragePlugin {
    fn storage() -> worktools_plugin_api::storage::PluginStorage {
        worktools_plugin_api::storage::PluginStorage::new("object-storage", "object-storage.json")
    }

    fn load_data() -> anyhow::Result<PluginData> {
        Self::storage().load_json()
    }

    fn save_data(data: &PluginData) -> anyhow::Result<()> {
        Self::storage().save_json(data)
    }

    /// 从 JSON params 中提取字符串参数
    fn get_connection_param<'a>(params: &'a Value, key: &str) -> anyhow::Result<&'a str> {
        params
            .get(key)
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 {} 参数", key))
    }

    fn find_connection_by_id(
        connections: &[ConnectionConfig],
        id: &str,
    ) -> Option<ConnectionConfig> {
        connections.iter().find(|c| c.id == id).cloned()
    }

    /// 清理 endpoint 字符串：去协议前缀、去尾部斜杠
    fn clean_endpoint(s: &str) -> String {
        s.trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string()
    }

    /// 验证连接配置的完整性
    fn validate_connection(conn: &ConnectionConfig) -> anyhow::Result<()> {
        if conn.region.is_empty() {
            anyhow::bail!("连接配置中 region 不能为空");
        }
        if conn.bucket.is_empty() {
            anyhow::bail!("连接配置中 bucket 不能为空");
        }
        Ok(())
    }

    /// 根据连接配置创建对应的云服务商客户端（工厂方法）
    ///
    /// ## Rust 知识点: Box<dyn Trait>
    /// `Box::new(OssClient::new(...))` 返回 `Box<dyn ObjectStoreProvider>`：
    /// - `Box` 分配在堆上（因为 trait 对象大小未知）
    /// - `dyn ObjectStoreProvider` 使用虚函数表（vtable）进行动态分发
    fn build_provider(conn: &ConnectionConfig) -> anyhow::Result<Box<dyn ObjectStoreProvider>> {
        // 解密存储的密钥
        let ak = crypto::decrypt(&conn.access_key);
        let sk = crypto::decrypt(&conn.secret_key);
        let ep = conn.endpoint.as_deref().unwrap_or("").to_string();

        match conn.provider.as_str() {
            "aliyun" => Ok(Box::new(OssClient::new(ak, sk, ep))),
            "tencent" => {
                if !ep.is_empty() {
                    Ok(Box::new(CosClient::new_with_endpoint(ak, sk, ep)))
                } else {
                    Ok(Box::new(CosClient::new(ak, sk, conn.region.clone())))
                }
            }
            other => anyhow::bail!("不支持的云服务商: {}", other),
        }
    }
}

impl Plugin for ObjectStoragePlugin {
    fn id(&self) -> &str {
        "object-storage"
    }
    fn name(&self) -> &str {
        "对象存储"
    }
    fn description(&self) -> &str {
        "管理阿里云OSS和腾讯云COS，支持文件浏览/上传/下载/搜索/删除"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn icon(&self) -> &str {
        "\u{1F4E6}"
    }

    fn get_view(&self) -> String {
        "<div>插件前端资源加载中...</div>".to_string()
    }

    /// 初始化时从文件加载持久化数据
    fn init(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let data = Self::load_data().unwrap_or_default();
        *self.data.lock().unwrap() = data;
        tracing::info!(
            connections = self.data.lock().unwrap().connections.len(),
            "对象存储插件初始化完成"
        );
        Ok(())
    }

    fn handle_call(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            // ── 连接管理 CRUD ──
            "add_connection" | "update_connection" => {
                let provider = Self::get_connection_param(&params, "provider")?.to_string();
                let name = Self::get_connection_param(&params, "name")?.to_string();
                let access_key = Self::get_connection_param(&params, "access_key")?.to_string();
                let secret_key = Self::get_connection_param(&params, "secret_key")?.to_string();
                let region = Self::get_connection_param(&params, "region")?.to_string();
                let bucket = Self::get_connection_param(&params, "bucket")?.to_string();
                let endpoint = params
                    .get("endpoint")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| Some(Self::clean_endpoint(s)))
                    .unwrap_or(None);

                let mut data = Self::load_data()?;

                match method {
                    "add_connection" => {
                        let conn = ConnectionConfig {
                            id: uuid::Uuid::new_v4().to_string(),
                            provider,
                            name,
                            access_key: crypto::encrypt(&access_key),
                            secret_key: crypto::encrypt(&secret_key),
                            region,
                            bucket,
                            endpoint,
                        };
                        let conn_id = conn.id.clone();
                        data.connections.push(conn);
                        Self::save_data(&data)?;
                        Ok(serde_json::json!({ "success": true, "id": conn_id }))
                    }
                    "update_connection" => {
                        let id = Self::get_connection_param(&params, "id")?.to_string();
                        let pos = data
                            .connections
                            .iter()
                            .position(|c| c.id == id)
                            .ok_or_else(|| anyhow::anyhow!("连接不存在"))?;
                        data.connections[pos] = ConnectionConfig {
                            id,
                            provider,
                            name,
                            access_key: crypto::encrypt(&access_key),
                            secret_key: crypto::encrypt(&secret_key),
                            region,
                            bucket,
                            endpoint,
                        };
                        Self::save_data(&data)?;
                        Ok(serde_json::json!({ "success": true }))
                    }
                    _ => unreachable!(),
                }
            }

            "list_connections" => {
                let data = Self::load_data()?;
                // 返回时脱敏：不返回 access_key 和 secret_key
                let connections: Vec<Value> = data
                    .connections
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "id": c.id, "provider": c.provider, "name": c.name,
                            "region": c.region, "bucket": c.bucket, "endpoint": c.endpoint,
                        })
                    })
                    .collect();
                Ok(serde_json::to_value(connections)?)
            }

            "get_connection" => {
                let id = Self::get_connection_param(&params, "id")?.to_string();
                let data = Self::load_data()?;
                let conn = Self::find_connection_by_id(&data.connections, &id)
                    .ok_or_else(|| anyhow::anyhow!("连接不存在"))?;
                // 单独获取时返回解密后的密钥
                Ok(serde_json::json!({
                    "id": conn.id, "provider": conn.provider, "name": conn.name,
                    "access_key": crypto::decrypt(&conn.access_key),
                    "secret_key": crypto::decrypt(&conn.secret_key),
                    "region": conn.region, "bucket": conn.bucket, "endpoint": conn.endpoint,
                }))
            }

            "delete_connection" => {
                let id = Self::get_connection_param(&params, "id")?.to_string();
                let mut data = Self::load_data()?;
                data.connections.retain(|c| c.id != id);
                Self::save_data(&data)?;
                Ok(serde_json::json!({ "success": true }))
            }

            // ── 对象操作 ──
            // 以下方法通过 build_provider 创建云服务商客户端，
            // 然后调用对应的方法完成操作。
            "list_buckets" | "list_objects" | "get_object_info" | "download_object"
            | "upload_object" | "delete_object" => {
                let conn_id = Self::get_connection_param(&params, "connection_id")?;
                let data = Self::load_data()?;
                let conn = Self::find_connection_by_id(&data.connections, conn_id)
                    .ok_or_else(|| anyhow::anyhow!("连接不存在"))?;
                Self::validate_connection(&conn)?;
                let provider = Self::build_provider(&conn)?;

                match method {
                    "list_buckets" => {
                        let buckets = provider.list_buckets(&conn.region)?;
                        Ok(serde_json::to_value(buckets)?)
                    }
                    "list_objects" => {
                        let bucket = Self::get_connection_param(&params, "bucket")?;
                        let prefix = params.get("prefix").and_then(|v| v.as_str()).unwrap_or("");
                        let delimiter = params.get("delimiter").and_then(|v| v.as_str());
                        let max_keys = params
                            .get("max_keys")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32);
                        let (objects, prefixes) = provider.list_objects(
                            bucket,
                            &conn.region,
                            prefix,
                            delimiter,
                            max_keys,
                        )?;
                        Ok(serde_json::json!({ "objects": objects, "prefixes": prefixes }))
                    }
                    "get_object_info" => {
                        let bucket = Self::get_connection_param(&params, "bucket")?;
                        let key = Self::get_connection_param(&params, "key")?;
                        let info = provider.head_object(bucket, &conn.region, key)?;
                        Ok(serde_json::to_value(info)?)
                    }
                    "download_object" => {
                        let bucket = Self::get_connection_param(&params, "bucket")?;
                        let key = Self::get_connection_param(&params, "key")?;
                        let file_path = Self::get_connection_param(&params, "file_path")?;
                        let bytes = provider.get_object(bucket, &conn.region, key)?;
                        std::fs::write(file_path, &bytes).context("写入文件失败")?;
                        tracing::info!(%key, size = bytes.len(), "下载对象成功");
                        Ok(serde_json::json!({ "success": true, "size": bytes.len() }))
                    }
                    "upload_object" => {
                        let bucket = Self::get_connection_param(&params, "bucket")?;
                        let key = Self::get_connection_param(&params, "key")?;
                        let file_path = Self::get_connection_param(&params, "file_path")?;
                        let data_bytes = std::fs::read(file_path).context("读取文件失败")?;
                        let mime = mime_guess_for_path(file_path);
                        provider.put_object(bucket, &conn.region, key, &data_bytes, &mime)?;
                        Ok(serde_json::json!({ "success": true }))
                    }
                    "delete_object" => {
                        let bucket = Self::get_connection_param(&params, "bucket")?;
                        let key = Self::get_connection_param(&params, "key")?;
                        provider.delete_object(bucket, &conn.region, key)?;
                        Ok(serde_json::json!({ "success": true }))
                    }
                    _ => unreachable!(),
                }
            }

            _ => Err(format!("未知方法: {method}").into()),
        }
    }
}

/// 根据文件扩展名猜测 MIME 类型
/// 用于上传时设置正确的 Content-Type
fn mime_guess_for_path(path: &str) -> String {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext.to_lowercase().as_str() {
        "html" | "htm" => "text/html".into(),
        "css" => "text/css".into(),
        "js" => "application/javascript".into(),
        "json" => "application/json".into(),
        "png" => "image/png".into(),
        "jpg" | "jpeg" => "image/jpeg".into(),
        "gif" => "image/gif".into(),
        "svg" => "image/svg+xml".into(),
        "pdf" => "application/pdf".into(),
        "zip" => "application/zip".into(),
        "txt" => "text/plain".into(),
        "xml" => "application/xml".into(),
        "yaml" | "yml" => "application/x-yaml".into(),
        _ => "application/octet-stream".into(), // 默认二进制流
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin = ObjectStoragePlugin {
        data: Mutex::new(PluginData::default()),
    };
    let boxed: Box<Box<dyn Plugin>> = Box::new(Box::new(plugin));
    Box::leak(boxed) as *mut Box<dyn Plugin>
}
