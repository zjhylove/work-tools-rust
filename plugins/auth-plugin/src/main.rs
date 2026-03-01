use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{stdin, stdout, BufRead, BufWriter, Write};
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

// 简化的 TOTP 生成 (模拟实现)
fn generate_totp(_secret: &str, digits: u32, _period: u64) -> Result<String> {
    // 简化实现:基于时间生成伪随机码
    use std::time::{SystemTime, UNIX_EPOCH};

    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // 每30秒变换一次
    let time_step = time / 30;
    let code = (time_step % 1_000_000) as u32;

    let width = digits as usize;
    Ok(format!("{:0width$}", code % (10_u32.pow(digits)), width = width))
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
