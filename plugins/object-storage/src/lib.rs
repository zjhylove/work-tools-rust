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

#[derive(Debug, Default, Serialize, Deserialize)]
struct PluginData {
    connections: Vec<ConnectionConfig>,
}

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

    fn get_connection_param<'a>(
        params: &'a Value,
        key: &str,
    ) -> anyhow::Result<&'a str> {
        params
            .get(key)
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 {} 参数", key))
    }

    fn find_connection_by_id(connections: &[ConnectionConfig], id: &str) -> Option<ConnectionConfig> {
        connections.iter().find(|c| c.id == id).cloned()
    }

    fn clean_endpoint(s: &str) -> String {
        s.trim().trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/').to_string()
    }

    fn validate_connection(conn: &ConnectionConfig) -> anyhow::Result<()> {
        if conn.region.is_empty() { anyhow::bail!("连接配置中 region 不能为空，请编辑连接补充"); }
        if conn.bucket.is_empty() { anyhow::bail!("连接配置中 bucket 不能为空，请编辑连接补充"); }
        Ok(())
    }

    fn build_provider(conn: &ConnectionConfig) -> anyhow::Result<Box<dyn ObjectStoreProvider>> {
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
            // -- 连接管理 --
            "add_connection" => {
                let provider = Self::get_connection_param(&params, "provider")?.to_string();
                let name = Self::get_connection_param(&params, "name")?.to_string();
                let access_key = Self::get_connection_param(&params, "access_key")?.to_string();
                let secret_key = Self::get_connection_param(&params, "secret_key")?.to_string();
                let region = Self::get_connection_param(&params, "region")?.to_string();
                let bucket = Self::get_connection_param(&params, "bucket")?.to_string();
                let endpoint = params.get("endpoint").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()).map(|s| Some(Self::clean_endpoint(s))).unwrap_or(None);

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

                let mut data = Self::load_data()?;
                data.connections.push(conn.clone());
                Self::save_data(&data)?;

                tracing::info!(conn_id = %conn.id, "添加云服务连接");
                Ok(serde_json::json!({ "success": true, "id": conn.id }))
            }

            "update_connection" => {
                let id = Self::get_connection_param(&params, "id")?.to_string();
                let provider = Self::get_connection_param(&params, "provider")?.to_string();
                let name = Self::get_connection_param(&params, "name")?.to_string();
                let access_key = Self::get_connection_param(&params, "access_key")?.to_string();
                let secret_key = Self::get_connection_param(&params, "secret_key")?.to_string();
                let region = Self::get_connection_param(&params, "region")?.to_string();
                let bucket = Self::get_connection_param(&params, "bucket")?.to_string();
                let endpoint = params.get("endpoint").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()).map(|s| Some(Self::clean_endpoint(s))).unwrap_or(None);

                let mut data = Self::load_data()?;
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

                tracing::info!(conn_id = %data.connections[pos].id, "更新云服务连接");
                Ok(serde_json::json!({ "success": true }))
            }

            "list_connections" => {
                let data = Self::load_data()?;
                let connections: Vec<Value> = data
                    .connections
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "id": c.id,
                            "provider": c.provider,
                            "name": c.name,
                            "region": c.region,
                            "bucket": c.bucket,
                            "endpoint": c.endpoint,
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
                Ok(serde_json::json!({
                    "id": conn.id,
                    "provider": conn.provider,
                    "name": conn.name,
                    "access_key": crypto::decrypt(&conn.access_key),
                    "secret_key": crypto::decrypt(&conn.secret_key),
                    "region": conn.region,
                    "bucket": conn.bucket,
                    "endpoint": conn.endpoint,
                }))
            }

            "delete_connection" => {
                let id = Self::get_connection_param(&params, "id")?.to_string();
                let mut data = Self::load_data()?;
                let before = data.connections.len();
                data.connections.retain(|c| c.id != id);
                Self::save_data(&data)?;
                tracing::info!(conn_id = %id, removed = before > data.connections.len(), "删除连接");
                Ok(serde_json::json!({ "success": true }))
            }

            "list_buckets" => {
                let conn_id = Self::get_connection_param(&params, "connection_id")?;
                let data = Self::load_data()?;
                let conn = Self::find_connection_by_id(&data.connections, conn_id)
                    .ok_or_else(|| anyhow::anyhow!("连接不存在"))?;
                Self::validate_connection(&conn)?;
                let provider = Self::build_provider(&conn)?;
                let buckets = provider.list_buckets(&conn.region)?;
                Ok(serde_json::to_value(buckets)?)
            }

            "list_objects" => {
                let conn_id = Self::get_connection_param(&params, "connection_id")?;
                let bucket = Self::get_connection_param(&params, "bucket")?;
                let prefix = params.get("prefix").and_then(|v| v.as_str()).unwrap_or("");
                let delimiter = params.get("delimiter").and_then(|v| v.as_str());
                let max_keys = params.get("max_keys").and_then(|v| v.as_u64()).map(|v| v as u32);

                let data = Self::load_data()?;
                let conn = Self::find_connection_by_id(&data.connections, conn_id)
                    .ok_or_else(|| anyhow::anyhow!("连接不存在"))?;
                Self::validate_connection(&conn)?;
                let provider = Self::build_provider(&conn)?;
                let (objects, prefixes) = provider.list_objects(bucket, &conn.region, prefix, delimiter, max_keys)?;
                Ok(serde_json::json!({ "objects": objects, "prefixes": prefixes }))
            }

            "get_object_info" => {
                let conn_id = Self::get_connection_param(&params, "connection_id")?;
                let bucket = Self::get_connection_param(&params, "bucket")?;
                let key = Self::get_connection_param(&params, "key")?;
                let data = Self::load_data()?;
                let conn = Self::find_connection_by_id(&data.connections, conn_id)
                    .ok_or_else(|| anyhow::anyhow!("连接不存在"))?;
                Self::validate_connection(&conn)?;
                let provider = Self::build_provider(&conn)?;
                let info = provider.head_object(bucket, &conn.region, key)?;
                Ok(serde_json::to_value(info)?)
            }

            "download_object" => {
                let conn_id = Self::get_connection_param(&params, "connection_id")?;
                let bucket = Self::get_connection_param(&params, "bucket")?;
                let key = Self::get_connection_param(&params, "key")?;
                let file_path = Self::get_connection_param(&params, "file_path")?;
                let data = Self::load_data()?;
                let conn = Self::find_connection_by_id(&data.connections, conn_id)
                    .ok_or_else(|| anyhow::anyhow!("连接不存在"))?;
                Self::validate_connection(&conn)?;
                let provider = Self::build_provider(&conn)?;
                let bytes = provider.get_object(bucket, &conn.region, key)?;
                std::fs::write(file_path, &bytes).context("写入文件失败")?;
                tracing::info!(%key, size = bytes.len(), path = file_path, "下载对象成功");
                Ok(serde_json::json!({ "success": true, "size": bytes.len() }))
            }

            "upload_object" => {
                let conn_id = Self::get_connection_param(&params, "connection_id")?;
                let bucket = Self::get_connection_param(&params, "bucket")?;
                let key = Self::get_connection_param(&params, "key")?;
                let file_path = Self::get_connection_param(&params, "file_path")?;
                let data = std::fs::read(file_path).context("读取文件失败")?;
                let mime = mime_guess_for_path(file_path);
                let storage_data = Self::load_data()?;
                let conn = Self::find_connection_by_id(&storage_data.connections, conn_id)
                    .ok_or_else(|| anyhow::anyhow!("连接不存在"))?;
                Self::validate_connection(&conn)?;
                let provider = Self::build_provider(&conn)?;
                provider.put_object(bucket, &conn.region, key, &data, &mime)?;
                tracing::info!(%key, size = data.len(), "上传对象成功");
                Ok(serde_json::json!({ "success": true }))
            }

            "delete_object" => {
                let conn_id = Self::get_connection_param(&params, "connection_id")?;
                let bucket = Self::get_connection_param(&params, "bucket")?;
                let key = Self::get_connection_param(&params, "key")?;
                let data = Self::load_data()?;
                let conn = Self::find_connection_by_id(&data.connections, conn_id)
                    .ok_or_else(|| anyhow::anyhow!("连接不存在"))?;
                Self::validate_connection(&conn)?;
                let provider = Self::build_provider(&conn)?;
                provider.delete_object(bucket, &conn.region, key)?;
                tracing::info!(%key, "删除对象成功");
                Ok(serde_json::json!({ "success": true }))
            }

            _ => Err(format!("未知方法: {method}").into()),
        }
    }
}

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
        "tar" => "application/x-tar".into(),
        "gz" => "application/gzip".into(),
        "txt" => "text/plain".into(),
        "mp3" => "audio/mpeg".into(),
        "mp4" => "video/mp4".into(),
        "xml" => "application/xml".into(),
        "yaml" | "yml" => "application/x-yaml".into(),
        _ => "application/octet-stream".into(),
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
