use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{stdin, stdout, BufRead, BufWriter, Write};
use std::path::PathBuf;
use worktools_rpc_protocol::RpcServer;

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

// 数据存储结构
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

// 获取数据文件路径
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

// 加载数据
fn load_data() -> Result<AuthData> {
    let data_path = get_data_file_path()?;

    if !data_path.exists() {
        return Ok(AuthData::default());
    }

    let file = File::open(&data_path)?;
    let data: AuthData = serde_json::from_reader(file)?;
    Ok(data)
}

// 保存数据
fn save_data(data: &AuthData) -> Result<()> {
    let data_path = get_data_file_path()?;

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

fn get_plugin_info() -> serde_json::Value {
    serde_json::json!({
        "id": "auth",
        "name": "双因素验证",
        "version": "1.0.0",
        "description": "TOTP 双因素认证验证器",
        "icon": "🔐"
    })
}

fn get_view_schema() -> serde_json::Value {
    serde_json::json!({
        "fields": [
            {
                "type": "input",
                "label": "账户名称",
                "key": "name",
                "placeholder": "例如: Google Account",
                "required": true,
                "minLength": 2
            },
            {
                "type": "input",
                "label": "发行方",
                "key": "issuer",
                "placeholder": "例如: Google",
                "required": true
            },
            {
                "type": "input",
                "label": "密钥",
                "key": "secret",
                "placeholder": "输入或生成密钥",
                "required": true,
                "minLength": 16
            },
            {
                "type": "input",
                "label": "算法",
                "key": "algorithm",
                "placeholder": "SHA1",
                "default": "SHA1"
            },
            {
                "type": "input",
                "label": "验证码位数",
                "key": "digits",
                "placeholder": "6",
                "default": "6",
                "required": true
            },
            {
                "type": "input",
                "label": "更新间隔(秒)",
                "key": "period",
                "placeholder": "30",
                "default": "30",
                "required": true
            },
            {
                "type": "button",
                "label": "💾 保存验证器",
                "key": "save"
            }
        ]
    })
}

// 真正的 TOTP 生成实现
fn generate_totp(secret: &str, digits: u32, period: u64) -> Result<String> {
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

fn generate_secret() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut rng = rand::thread_rng();

    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

fn generate_qr_code(entry: &AuthEntry) -> String {
    // 生成 otpauth URI
    let uri = format!(
        "otpauth://totp/{}:{}?secret={}&algorithm={}&digits={}&period={}",
        entry.issuer, entry.name, entry.secret, entry.algorithm, entry.digits, entry.period
    );

    // 返回 QR Code URL
    format!("https://api.qrserver.com/v1/create-qr-code/?size=300x300&data={}", urlencoding::encode(&uri))
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Auth Plugin 启动");

    let mut rpc_server = RpcServer::new();

    // 注册 get_info 处理器
    rpc_server.register_handler("get_info", |_params| {
        let info = get_plugin_info();
        Ok(serde_json::to_value(info)?)
    });

    // 注册 get_view 处理器
    rpc_server.register_handler("get_view", |_params| {
        let schema = get_view_schema();
        Ok(serde_json::to_value(schema)?)
    });

    // 注册 generate_totp 处理器
    rpc_server.register_handler("generate_totp", |params| {
        let secret = params.get("secret").and_then(|v| v.as_str()).unwrap_or("");
        let digits = params.get("digits").and_then(|v| v.as_u64()).unwrap_or(6) as u32;
        let period = params.get("period").and_then(|v| v.as_u64()).unwrap_or(30);

        let code = generate_totp(secret, digits, period).unwrap_or_else(|_| "000000".to_string());
        Ok(serde_json::json!({ "code": code }))
    });

    // 注册 generate_secret 处理器
    rpc_server.register_handler("generate_secret", |_params| {
        let secret = generate_secret();
        Ok(serde_json::json!({ "secret": secret }))
    });

    // 注册 generate_qr_code 处理器
    rpc_server.register_handler("generate_qr_code", |params| {
        let entry: AuthEntry = serde_json::from_value(serde_json::to_value(params)?)?;
        let qr_url = generate_qr_code(&entry);
        Ok(serde_json::json!({ "qr_url": qr_url }))
    });

    // 注册 list_entries 处理器 - 列出所有认证条目
    rpc_server.register_handler("list_entries", |_params| {
        let data = load_data()?;
        Ok(serde_json::json!({ "entries": data.entries }))
    });

    // 注册 add_entry 处理器 - 添加认证条目
    rpc_server.register_handler("add_entry", |params| {
        let mut entry: AuthEntry = serde_json::from_value(serde_json::to_value(params)?)?;

        // 设置创建时间
        entry.created_at = chrono::Utc::now().to_rfc3339();

        // 生成 ID
        entry.id = uuid::Uuid::new_v4().to_string();

        // 加载现有数据
        let mut data = load_data()?;

        // 添加新条目
        data.entries.push(entry.clone());

        // 保存数据
        save_data(&data)?;

        Ok(serde_json::to_value(entry)?)
    });

    // 注册 update_entry 处理器 - 更新认证条目
    rpc_server.register_handler("update_entry", |params| {
        let updated_entry: AuthEntry = serde_json::from_value(serde_json::to_value(params)?)?;

        // 加载现有数据
        let mut data = load_data()?;

        // 查找并更新条目
        let index = data.entries
            .iter()
            .position(|e| e.id == updated_entry.id)
            .ok_or_else(|| anyhow::anyhow!("条目不存在"))?;

        // 保留创建时间
        let created_at = data.entries[index].created_at.clone();
        let mut entry = updated_entry;
        entry.created_at = created_at;

        data.entries[index] = entry.clone();

        // 保存数据
        save_data(&data)?;

        Ok(serde_json::to_value(entry)?)
    });

    // 注册 delete_entry 处理器 - 删除认证条目
    rpc_server.register_handler("delete_entry", |params| {
        let id = params.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 id 参数"))?;

        // 加载现有数据
        let mut data = load_data()?;

        // 查找并删除条目
        let index = data.entries
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(|| anyhow::anyhow!("条目不存在"))?;

        data.entries.remove(index);

        // 保存数据
        save_data(&data)?;

        Ok(serde_json::json!({ "success": true }))
    });

    // 从 stdin 读取 JSON-RPC 请求,处理并输出响应
    let stdin = stdin();
    let stdout = stdout();
    let mut stdout = BufWriter::new(stdout.lock());

    for line in stdin.lock().lines() {
        let line = line?;
        tracing::info!("收到请求: {}", line);

        let response = rpc_server.handle(&line);
        writeln!(stdout, "{}", response)?;
        stdout.flush()?;
    }

    Ok(())
}
