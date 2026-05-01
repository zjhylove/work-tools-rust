//! # 密码管理器插件
//!
//! 本地加密存储和管理密码。实现 Plugin trait，通过动态库加载到主程序。
//!
//! ## 数据流
//! ```
//! 用户输入 → 前端 JS → pluginAPI.call("add_password", {...})
//!   → Tauri IPC → PluginManager → handle_call("add_password", params)
//!   → AES 加密密码 → JSON 文件存储
//! ```
//!
//! ## Rust 知识点
//! - `once_cell::sync::Lazy`: 延迟初始化的全局单例，线程安全
//! - `#[no_mangle]`: 禁止 Rust 的名称修饰，使 C 代码可以找到这个函数
//! - `extern "C"`: 使用 C 语言调用约定（ABI）
//! - `Box::leak`: 故意"泄漏"内存，将所有权转移给调用方
//! - `serde_json::json!`: 创建 JSON 值的宏

use anyhow::Result;
use serde::{Deserialize, Serialize};
use worktools_plugin_api::{Plugin, storage::PluginStorage};
use serde_json::Value;
mod crypto;
use crypto::{PasswordEncryptor, CryptoConfig};
use once_cell::sync::Lazy;

/// 密码条目（加密存储版本）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordEntry {
    pub id: String,            // UUID，唯一标识每个密码条目
    pub url: Option<String>,   // 关联的网址（可选）
    pub service: String,       // 服务名称
    pub username: String,      // 用户名
    pub password: String,      // 加密后的密码（存储为十六进制字符串）
    pub created_at: String,    // 创建时间（RFC 3339 格式）
    pub updated_at: Option<String>, // 最后更新时间
}

/// 数据存储结构（顶层 JSON 结构）
/// `#[derive(Default)]` 生成 Default 实现：`PasswordData { entries: Vec::new() }`
#[derive(Debug, Default, Serialize, Deserialize)]
struct PasswordData {
    entries: Vec<PasswordEntry>,
}

/// 密码管理器插件
/// 这是一个空结构体（unit-like struct），不包含任何字段
/// 插件状态通过 `PluginStorage`（文件系统）管理，不需要内存中的字段
pub struct PasswordManager;

/// 全局共享的加密器实例（单例模式，避免重复创建）
///
/// ## Rust 知识点: Lazy 单例
/// `Lazy<T>` 在第一次访问时才初始化，之后返回同一个实例。
/// 这比 `static mut` + `unsafe` 更安全和方便。
/// `static` 变量在整个程序运行期间存在，`Lazy` 保证线程安全的延迟初始化。
static ENCRYPTOR: Lazy<PasswordEncryptor> = Lazy::new(|| {
    PasswordEncryptor::new(CryptoConfig::default())
});

impl PasswordManager {
    /// 获取数据存储实例
    fn storage() -> PluginStorage {
        PluginStorage::new("password-manager", "password-manager.json")
    }

    /// 获取加密器实例（返回全局单例）
    /// 返回 `&'static` 引用 — 指向程序生命周期内一直存在的值
    fn encryptor() -> &'static PasswordEncryptor {
        &ENCRYPTOR
    }

    /// 加密密码，失败时返回明文（优雅降级）
    fn encrypt_or_plain(password: &str) -> String {
        Self::encryptor()
            .encrypt_password(password)
            .unwrap_or_else(|_| password.to_string()) // 降级：加密失败则存明文
    }

    /// 解密密码，失败时返回原始值
    fn decrypt_or_original(encrypted: &str) -> String {
        Self::encryptor()
            .decrypt_password(encrypted)
            .unwrap_or_else(|_| encrypted.to_string())
    }

    /// 加载数据
    fn load_data() -> Result<PasswordData> {
        Self::storage().load_json()
    }

    /// 保存数据（保留 salt 和 validation_token 字段不被覆盖）
    fn save_data(data: &PasswordData) -> Result<()> {
        Self::storage().save_json_preserving(data, &["salt", "validation_token"])
    }
}

/// 实现 Plugin trait — 密码管理器的核心行为
impl Plugin for PasswordManager {
    fn id(&self) -> &str { "password-manager" }
    fn name(&self) -> &str { "密码管理器" }
    fn description(&self) -> &str { "本地安全存储和管理密码" }
    fn version(&self) -> &str { "1.0.0" }
    fn icon(&self) -> &str { "🔐" }

    fn get_view(&self) -> String {
        // 插件使用独立前端资源（assets/index.html），这个方法只是占位符
        "<div>插件前端资源加载中...</div>".to_string()
    }

    /// 处理来自前端的方法调用
    ///
    /// ## Rust 知识点: 模式匹配 + 错误处理
    /// `match method { ... _ => Err(...) }` 确保所有方法都被处理。
    /// `?` 操作符在 Ok 时解包，在 Err 时立即返回错误。
    fn handle_call(&mut self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            // ── 列出所有密码 ──
            "list_passwords" => {
                let data = Self::load_data()?;
                // 在返回给前端前解密密码
                let entries: Vec<Value> = data.entries.into_iter().map(|entry| {
                    let password = Self::decrypt_or_original(&entry.password);

                    serde_json::json!({
                        "id": entry.id,
                        "url": entry.url.as_deref().unwrap_or_default(),
                        "service": entry.service,
                        "username": entry.username,
                        "password": password, // 解密后的明文（仅在前端显示）
                        "created_at": entry.created_at,
                        "updated_at": entry.updated_at.as_deref().unwrap_or_default(),
                    })
                }).collect();
                // 直接返回数组（不包装在对象中），前端可以直接遍历
                Ok(serde_json::to_value(entries)?)
            }

            // ── 添加密码 ──
            "add_password" => {
                // 从 JSON params 中提取字段
                // `and_then(|v| v.as_str())` 先检查是否为字符串，再取出值
                let service = params.get("service")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 service 参数"))?;

                let username = params.get("username")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 username 参数"))?;

                let password = params.get("password")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 password 参数"))?;

                let url = params.get("url").and_then(|v| v.as_str());

                // 加密后存储
                let encrypted_password = Self::encrypt_or_plain(password);

                let entry = PasswordEntry {
                    id: uuid::Uuid::new_v4().to_string(), // 生成唯一 ID
                    url: url.map(|s| s.to_string()),
                    service: service.to_string(),
                    username: username.to_string(),
                    password: encrypted_password,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    updated_at: None,
                };

                let mut data = Self::load_data()?;
                data.entries.push(entry.clone());
                Self::save_data(&data)?;

                tracing::info!(service = %entry.service, "添加密码条目");

                // 返回明文密码给前端（方便用户确认）
                Ok(serde_json::json!({
                    "id": entry.id,
                    "url": entry.url.as_deref().unwrap_or_default(),
                    "service": entry.service,
                    "username": entry.username,
                    "password": password,
                    "created_at": entry.created_at,
                    "updated_at": entry.updated_at.as_deref().unwrap_or_default(),
                }))
            }

            // ── 更新密码 ──
            "update_password" => {
                let id = params.get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 id 参数"))?;
                let service = params.get("service")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 service 参数"))?;
                let username = params.get("username")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 username 参数"))?;
                let password = params.get("password")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 password 参数"))?;
                let url = params.get("url").and_then(|v| v.as_str());

                let encrypted_password = Self::encrypt_or_plain(password);

                let mut data = Self::load_data()?;
                // `position` 查找元素在 Vec 中的索引
                let index = data.entries
                    .iter()
                    .position(|e| e.id == id)
                    .ok_or_else(|| anyhow::anyhow!("密码条目不存在"))?;

                let created_at = data.entries[index].created_at.clone();

                let entry = PasswordEntry {
                    id: id.to_string(),
                    url: url.map(|s| s.to_string()),
                    service: service.to_string(),
                    username: username.to_string(),
                    password: encrypted_password,
                    created_at,
                    updated_at: Some(chrono::Utc::now().to_rfc3339()),
                };

                data.entries[index] = entry.clone();
                Self::save_data(&data)?;

                Ok(serde_json::json!({
                    "id": entry.id,
                    "password": password,
                    // ... 其他字段
                    "service": entry.service,
                    "username": entry.username,
                }))
            }

            // ── 删除密码 ──
            "delete_password" => {
                let id = params.get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 id 参数"))?;

                let mut data = Self::load_data()?;
                let index = data.entries
                    .iter()
                    .position(|e| e.id == id)
                    .ok_or_else(|| anyhow::anyhow!("密码条目不存在"))?;

                data.entries.remove(index); // 按索引删除
                Self::save_data(&data)?;

                Ok(serde_json::json!({ "success": true }))
            }

            // ── 清空所有密码 ──
            "clear_all_passwords" => {
                let mut data = Self::load_data()?;
                let count = data.entries.len();
                data.entries.clear(); // 清空 Vec
                Self::save_data(&data)?;
                tracing::warn!(count, "清空所有密码条目");
                Ok(serde_json::json!({ "success": true }))
            }

            // ── 导出密码 ──
            "export_passwords" => {
                let data = Self::load_data()?;
                let json = serde_json::to_string_pretty(&data)?;
                Ok(serde_json::json!({ "data": json }))
            }

            // ── 导入密码 ──
            "import_passwords" => {
                let json_data = params.get("data")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 data 参数"))?;

                let imported_data: PasswordData = serde_json::from_str(json_data)?;
                let mut data = Self::load_data()?;

                // 只导入不存在的条目（按 ID 去重）
                for entry in &imported_data.entries {
                    if !data.entries.iter().any(|e| e.id == entry.id) {
                        data.entries.push(entry.clone());
                    }
                }

                Self::save_data(&data)?;
                Ok(serde_json::json!({ "success": true }))
            }

            _ => Err(format!("未知方法: {method}").into()),
        }
    }
}

/// 插件工厂函数 — 动态库导出的入口
///
/// ## Rust 知识点: FFI 导出
/// - `#[no_mangle]`: 禁用 Rust 的名称修饰，使符号名保持 `plugin_create`
/// - `pub extern "C"`: 使用 C ABI，确保与主程序的 libloading 兼容
/// - `*mut Box<dyn Plugin>`: 返回原始指针（C 兼容）
///
/// ## 为什么用两层 Box + leak？
/// 1. 外层 `Box<Box<dyn Plugin>>`: fat pointer（数据指针 + vtable 指针）
/// 2. 内层 `Box<dyn Plugin>`: 实际的 trait 对象在堆上
/// 3. `Box::leak`: 防止 drop，将所有权转移给调用方
///
/// 调用方（PluginManager）使用 `Box::from_raw(ptr)` 重新获得所有权。
#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(PasswordManager));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
