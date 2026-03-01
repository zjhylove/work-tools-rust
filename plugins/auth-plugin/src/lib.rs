use anyhow::Result;
use worktools_plugin_api::Plugin;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

/// Auth Entry - 双因素认证条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthEntry {
    pub id: String,
    pub name: String,
    pub issuer: String,
    pub secret: String,
    pub algorithm: String,
    pub digits: u32,
    pub period: u64,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// 数据存储结构
#[derive(Debug, Serialize, Deserialize)]
struct AuthData {
    entries: Vec<AuthEntry>,
}

impl Default for AuthData {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

/// Auth Plugin - 双因素认证
pub struct AuthPlugin;

impl AuthPlugin {
    /// 获取数据文件路径
    fn get_data_file_path() -> Result<PathBuf> {
        let mut data_dir = dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("无法获取数据目录"))?;
        data_dir.push("worktools");
        data_dir.push("data");

        // 创建目录(如果不存在)
        std::fs::create_dir_all(&data_dir)?;

        data_dir.push("auth.json");
        Ok(data_dir)
    }

    /// 加载数据
    fn load_data() -> Result<AuthData> {
        let data_path = Self::get_data_file_path()?;

        if !data_path.exists() {
            return Ok(AuthData::default());
        }

        let file = File::open(&data_path)?;
        let data: AuthData = serde_json::from_reader(file)?;
        Ok(data)
    }

    /// 保存数据
    fn save_data(data: &AuthData) -> Result<()> {
        let data_path = Self::get_data_file_path()?;

        // 使用临时文件模式确保原子性写入
        let temp_path = data_path.with_extension("tmp");
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)?;

        serde_json::to_writer_pretty(&file, data)?;
        file.sync_all()?;

        // 原子性替换文件
        std::fs::rename(&temp_path, &data_path)?;

        Ok(())
    }

    /// 生成 TOTP 验证码
    fn generate_totp_internal(secret: &str, digits: u32, period: u64) -> Result<String> {
        use std::time::{SystemTime, UNIX_EPOCH};
        use hmac::{Hmac, Mac};
        use sha1::Sha1;
        use base32::Alphabet;

        type HmacSha1 = Hmac<Sha1>;

        // 清理密钥(移除空格和转换为大写)
        let secret_clean = secret.replace(" ", "").to_uppercase();

        // Base32 解码
        let secret_bytes = base32::decode(Alphabet::Rfc4648 { padding: true }, &secret_clean)
            .ok_or_else(|| anyhow::anyhow!("无效的 Base32 密钥"))?;

        // 获取当前时间步
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let time_step = time / period;

        // 将时间步转换为 8 字节数组(大端序)
        let time_bytes: [u8; 8] = time_step.to_be_bytes();

        // 计算 HMAC-SHA1
        let mut mac = HmacSha1::new_from_slice(&secret_bytes)
            .map_err(|e| anyhow::anyhow!("HMAC 初始化失败: {}", e))?;
        mac.update(&time_bytes);
        let hash = mac.finalize().into_bytes();

        // 动态截取
        let offset = (hash[hash.len() - 1] & 0x0f) as usize;
        let binary = ((hash[offset] & 0x7f) as u32) << 24
            | ((hash[offset + 1] & 0xff) as u32) << 16
            | ((hash[offset + 2] & 0xff) as u32) << 8
            | (hash[offset + 3] & 0xff) as u32;

        // 取模并格式化
        let code = binary % 10_u32.pow(digits);
        let width = digits as usize;
        Ok(format!("{:0width$}", code, width = width))
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
        "🔐"
    }

    fn get_view(&self) -> String {
        r#"
        <div id="auth-app">
            <h2>双因素验证</h2>
            <p>Auth Plugin (开发中)</p>
        </div>
        "#.to_string()
    }

    fn handle_call(&mut self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match method {
            "list_entries" => {
                let data = Self::load_data()?;
                let entries: Value = serde_json::to_value(data.entries)?;
                Ok(serde_json::json!({ "entries": entries }))
            }
            "add_entry" => {
                let entry: AuthEntry = serde_json::from_value(params)
                    .map_err(|e| anyhow::anyhow!("解析参数失败: {}", e))?;

                let mut data = Self::load_data()?;
                data.entries.push(entry.clone());
                Self::save_data(&data)?;

                Ok(serde_json::to_value(entry)?)
            }
            "update_entry" => {
                let entry: AuthEntry = serde_json::from_value(params)
                    .map_err(|e| anyhow::anyhow!("解析参数失败: {}", e))?;

                let mut data = Self::load_data()?;
                let index = data.entries
                    .iter()
                    .position(|e| e.id == entry.id)
                    .ok_or_else(|| anyhow::anyhow!("条目不存在"))?;

                data.entries[index] = entry.clone();
                Self::save_data(&data)?;

                Ok(serde_json::to_value(entry)?)
            }
            "delete_entry" => {
                let id = params.get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 id 参数"))?;

                let mut data = Self::load_data()?;
                let index = data.entries
                    .iter()
                    .position(|e| e.id == id)
                    .ok_or_else(|| anyhow::anyhow!("条目不存在"))?;

                data.entries.remove(index);
                Self::save_data(&data)?;

                Ok(serde_json::json!({ "success": true }))
            }
            "generate_totp" => {
                let secret = params.get("secret")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 secret 参数"))?;

                let digits = params.get("digits")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| anyhow::anyhow!("缺少 digits 参数"))? as u32;

                let period = params.get("period")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| anyhow::anyhow!("缺少 period 参数"))?;

                let code = Self::generate_totp_internal(secret, digits, period)?;

                Ok(serde_json::json!({ "code": code }))
            }
            _ => Err(format!("未知方法: {}", method).into()),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(AuthPlugin));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
