//! # 双因素认证 (TOTP) 插件
//!
//! 基于时间的一次性密码 (TOTP) 生成器，遵循 RFC 6238。
//! 可以生成类似于 Google Authenticator 的 6 位验证码。
//!
//! ## TOTP 算法原理
//! 1. 将密钥（Base32 编码）解码为字节
//! 2. (当前时间 / 时间周期) 作为计数器
//! 3. HMAC-SHA1(密钥, 计数器) → 哈希值
//! 4. 动态截取 → 6 位数字验证码
//!
//! ## Rust 知识点
//! - `hmac` crate: HMAC 消息认证码
//! - `base32` crate: Base32 编解码（TOTP 密钥的标准编码）
//! - `getrandom` crate: 安全的随机数生成器
//! - `SystemTime::now().duration_since(UNIX_EPOCH)`: 获取 Unix 时间戳

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use worktools_plugin_api::{storage::PluginStorage, Plugin};

/// 双因素认证条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthEntry {
    pub id: String,        // UUID
    pub name: String,      // 显示名称（如 "GitHub"）
    pub issuer: String,    // 发行方（如 "github.com"）
    pub secret: String,    // Base32 编码的密钥
    pub algorithm: String, // 算法（SHA1 / SHA256 / SHA512）
    pub digits: u32,       // 验证码位数（通常是 6）
    pub period: u64,       // 时间周期（秒，通常是 30）
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")] // None 时不序列化此字段
    pub updated_at: Option<String>,
}

/// 数据存储结构
#[derive(Debug, Default, Serialize, Deserialize)]
struct AuthData {
    entries: Vec<AuthEntry>,
}

/// 双因素认证插件
pub struct AuthPlugin;

impl AuthPlugin {
    fn storage() -> PluginStorage {
        PluginStorage::new("auth", "auth.json")
    }

    fn load_data() -> Result<AuthData> {
        Self::storage().load_json()
    }

    fn save_data(data: &AuthData) -> Result<()> {
        Self::storage().save_json(data)
    }

    /// 生成 TOTP 验证码（核心算法）
    ///
    /// ## Rust 知识点: 内部导入
    /// 函数内使用 `use` 导入 — 将依赖范围限制在最小。
    /// 这样做的优势：如果函数被删除，相关的导入也会被清理，减少无用依赖。
    fn generate_totp_internal(secret: &str, digits: u32, period: u64) -> Result<String> {
        use base32::Alphabet;
        use hmac::{Hmac, Mac};
        use sha1::Sha1;
        use std::time::{SystemTime, UNIX_EPOCH};

        // 类型别名：HMAC-SHA1 的完整类型
        type HmacSha1 = Hmac<Sha1>;

        // 1. 清理密钥：移除空格，转为大写
        let secret_clean = secret.replace(" ", "").to_uppercase();

        // 2. Base32 解码：将密钥转为原始字节
        let secret_bytes = base32::decode(Alphabet::Rfc4648 { padding: true }, &secret_clean)
            .ok_or_else(|| anyhow::anyhow!("无效的 Base32 密钥"))?;

        // 3. 计算时间步：当前 Unix 时间 / 时间周期
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH) // 自 1970-01-01 以来的时间
            .unwrap()
            .as_secs();
        let time_step = time / period;

        // 4. 将时间步转为 8 字节大端序数组
        let time_bytes: [u8; 8] = time_step.to_be_bytes();

        // 5. 计算 HMAC-SHA1(密钥, 时间步)
        let mut mac = HmacSha1::new_from_slice(&secret_bytes)
            .map_err(|e| anyhow::anyhow!("HMAC 初始化失败: {}", e))?;
        mac.update(&time_bytes);
        let hash = mac.finalize().into_bytes(); // 20 字节的哈希值

        // 6. 动态截取 (Dynamic Truncation)
        //    取哈希最后 4 位作为偏移量
        let offset = (hash[hash.len() - 1] & 0x0f) as usize;
        //    从 offset 处取 4 字节，去掉最高位（& 0x7f）
        let binary = ((hash[offset] & 0x7f) as u32) << 24
            | (hash[offset + 1] as u32) << 16
            | (hash[offset + 2] as u32) << 8
            | hash[offset + 3] as u32;

        // 7. 取模得到指定位数的验证码
        let code = binary % 10_u32.pow(digits);
        // 格式化为指定位数（左侧补零）
        Ok(format!("{:0width$}", code, width = digits as usize))
    }
}

impl Plugin for AuthPlugin {
    fn id(&self) -> &str {
        "auth"
    }
    fn name(&self) -> &str {
        "双因素验证"
    }
    fn description(&self) -> &str {
        "TOTP 双因素认证"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn icon(&self) -> &str {
        "🔢"
    }
    fn get_view(&self) -> String {
        "<div>插件前端资源加载中...</div>".to_string()
    }

    fn handle_call(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            "list_entries" => {
                let data = Self::load_data()?;
                Ok(serde_json::to_value(data.entries)?)
            }

            "add_entry" | "update_entry" => {
                // 从 params 中提取完整的 entry 对象
                let entry_value = params
                    .get("entry")
                    .ok_or_else(|| anyhow::anyhow!("缺少 entry 参数"))?;

                let mut entry: AuthEntry = serde_json::from_value(entry_value.clone())
                    .map_err(|e| anyhow::anyhow!("解析参数失败: {}", e))?;

                // 自动生成 UUID（如果前端没提供）
                if entry.id.is_empty() {
                    entry.id = uuid::Uuid::new_v4().to_string();
                }

                let mut data = Self::load_data()?;

                match method {
                    "add_entry" => data.entries.push(entry.clone()),
                    "update_entry" => {
                        let index = data
                            .entries
                            .iter()
                            .position(|e| e.id == entry.id)
                            .ok_or_else(|| anyhow::anyhow!("条目不存在"))?;
                        data.entries[index] = entry.clone();
                    }
                    _ => unreachable!(), // 编译器可以证明这里不会被执行
                }

                Self::save_data(&data)?;
                Ok(serde_json::to_value(entry)?)
            }

            "delete_entry" => {
                let id = params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 id 参数"))?;

                let mut data = Self::load_data()?;
                let index = data
                    .entries
                    .iter()
                    .position(|e| e.id == id)
                    .ok_or_else(|| anyhow::anyhow!("条目不存在"))?;
                data.entries.remove(index);
                Self::save_data(&data)?;

                Ok(serde_json::json!({ "success": true }))
            }

            // ── 生成新的随机密钥 ──
            "generate_secret" => {
                // 生成 20 字节（160 位）的随机密钥
                let mut secret_bytes = [0u8; 20];
                getrandom::getrandom(&mut secret_bytes)
                    .map_err(|e| anyhow::anyhow!("生成随机数失败: {}", e))?;
                // 编码为 Base32（TOTP 标准编码格式）
                let secret =
                    base32::encode(base32::Alphabet::Rfc4648 { padding: true }, &secret_bytes);
                Ok(serde_json::to_value(secret)?)
            }

            // ── 生成验证码 ──
            "generate_totp" => {
                let secret = params
                    .get("secret")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 secret 参数"))?;
                let digits = params
                    .get("digits")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| anyhow::anyhow!("缺少 digits 参数"))?
                    as u32;
                let period = params
                    .get("period")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| anyhow::anyhow!("缺少 period 参数"))?;

                let code = Self::generate_totp_internal(secret, digits, period)?;
                Ok(serde_json::json!({ "code": code }))
            }

            _ => Err(format!("未知方法: {method}").into()),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(AuthPlugin));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
