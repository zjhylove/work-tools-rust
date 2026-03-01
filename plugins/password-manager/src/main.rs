use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use worktools_shared_types::{PluginInfo, ViewSchema};
use worktools_rpc_protocol::RpcServer;

/// 密码条目 (加密版本)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordEntry {
    pub id: String,
    pub url: Option<String>,
    pub service: String,
    pub username: String,
    pub password: String, // 存储已加密的密码
    pub created_at: String,
    pub updated_at: Option<String>,
}

/// 数据存储结构
#[derive(Debug, Serialize, Deserialize)]
struct PasswordData {
    entries: Vec<PasswordEntry>,
}

impl Default for PasswordData {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

// 获取数据文件路径
fn get_data_file_path() -> Result<PathBuf> {
    // 使用与 Tauri 应用相同的路径: ~/.worktools/history/plugins/
    let home = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("无法获取用户主目录"))?;

    let mut data_dir = std::path::PathBuf::from(home);
    data_dir.push(".worktools/history/plugins");

    // 创建目录(如果不存在)
    std::fs::create_dir_all(&data_dir)?;

    data_dir.push("password-manager.json");
    Ok(data_dir)
}

// 加载数据
fn load_data() -> Result<PasswordData> {
    let data_path = get_data_file_path()?;

    if !data_path.exists() {
        return Ok(PasswordData::default());
    }

    let file = File::open(&data_path)?;
    let data: PasswordData = serde_json::from_reader(file)?;
    Ok(data)
}

// 保存数据
fn save_data(data: &PasswordData) -> Result<()> {
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

/// 插件信息
fn get_plugin_info() -> PluginInfo {
    PluginInfo {
        id: "password-manager".to_string(),
        name: "密码管理器".to_string(),
        version: "1.0.0".to_string(),
        description: "本地安全存储和管理密码".to_string(),
        icon: "🔐".to_string(),
    }
}

/// UI Schema - 前端使用独立组件
fn get_view_schema() -> ViewSchema {
    ViewSchema {
        fields: vec![],
    }
}

fn main() -> Result<()> {
    // 配置日志输出到 stderr,保持 stdout 纯净用于 JSON-RPC
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Password Manager Plugin 启动");

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

    // 注册 list_passwords 处理器 - 列出所有密码(返回加密数据)
    rpc_server.register_handler("list_passwords", |_params| {
        let data = load_data()?;

        // 手动构建 JSON,将 Option 字段转换为空字符串
        let entries: Vec<serde_json::Value> = data.entries.into_iter().map(|entry| {
            serde_json::json!({
                "id": entry.id,
                "url": entry.url.as_ref().unwrap_or(&String::new()),
                "service": entry.service,
                "username": entry.username,
                "password": entry.password,
                "created_at": entry.created_at,
                "updated_at": entry.updated_at.as_ref().unwrap_or(&String::new()),
            })
        }).collect();

        Ok(serde_json::json!({ "entries": entries }))
    });

    // 注册 get_password_detail 处理器 - 获取密码详情(返回加密数据)
    rpc_server.register_handler("get_password_detail", |params| {
        let id = params.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 id 参数"))?;

        let data = load_data()?;
        let entry = data.entries
            .iter()
            .find(|e| e.id == id)
            .ok_or_else(|| anyhow::anyhow!("密码条目不存在"))?;

        Ok(serde_json::json!({
            "id": entry.id,
            "url": entry.url.as_ref().unwrap_or(&String::new()),
            "service": entry.service,
            "username": entry.username,
            "password": entry.password, // 返回解密后的密码
            "created_at": entry.created_at,
            "updated_at": entry.updated_at.as_ref().unwrap_or(&String::new()),
        }))
    });

    // 注册 add_password 处理器 - 添加新密码(接收已加密的数据)
    rpc_server.register_handler("add_password", |params| {
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

        let entry = PasswordEntry {
            id: uuid::Uuid::new_v4().to_string(),
            url: url.map(|s| s.to_string()), // 这里的 None 会被序列化为 null
            service: service.to_string(),
            username: username.to_string(),
            password: password.to_string(), // 密码已在前端加密
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: None,
        };

        // 保存到数据
        let mut data = load_data()?;
        data.entries.push(entry.clone());
        save_data(&data)?;

        // 返回条目,确保 url 和 updated_at 字段是字符串而不是 null
        Ok(serde_json::json!({
            "id": entry.id,
            "url": entry.url.as_ref().unwrap_or(&String::new()),
            "service": entry.service,
            "username": entry.username,
            "password": entry.password,
            "created_at": entry.created_at,
            "updated_at": entry.updated_at.as_ref().unwrap_or(&String::new()),
        }))
    });

    // 注册 update_password 处理器 - 更新密码(接收已加密的数据)
    rpc_server.register_handler("update_password", |params| {
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

        let mut data = load_data()?;
        let index = data.entries
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(|| anyhow::anyhow!("密码条目不存在"))?;

        // 保留创建时间
        let created_at = data.entries[index].created_at.clone();

        let entry = PasswordEntry {
            id: id.to_string(),
            url: url.map(|s| s.to_string()),
            service: service.to_string(),
            username: username.to_string(),
            password: password.to_string(), // 密码已在前端加密
            created_at,
            updated_at: Some(chrono::Utc::now().to_rfc3339()),
        };

        data.entries[index] = entry.clone();
        save_data(&data)?;

        // 返回条目,确保 url 和 updated_at 字段格式正确
        Ok(serde_json::json!({
            "id": entry.id,
            "url": entry.url.as_ref().unwrap_or(&String::new()),
            "service": entry.service,
            "username": entry.username,
            "password": entry.password,
            "created_at": entry.created_at,
            "updated_at": entry.updated_at.as_ref().unwrap_or(&String::new()),
        }))
    });

    // 注册 delete_password 处理器 - 删除密码
    rpc_server.register_handler("delete_password", |params| {
        let id = params.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 id 参数"))?;

        let mut data = load_data()?;
        let index = data.entries
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(|| anyhow::anyhow!("密码条目不存在"))?;

        data.entries.remove(index);
        save_data(&data)?;

        Ok(serde_json::json!({ "success": true }))
    });

    // 注册 clear_all_passwords 处理器 - 清空所有密码
    rpc_server.register_handler("clear_all_passwords", |_params| {
        let mut data = load_data()?;
        data.entries.clear();
        save_data(&data)?;
        Ok(serde_json::json!({ "success": true }))
    });

    // 注册 export_passwords 处理器 - 导出密码
    rpc_server.register_handler("export_passwords", |_params| {
        let data = load_data()?;
        let json = serde_json::to_string_pretty(&data)?;
        Ok(serde_json::json!({ "data": json }))
    });

    // 注册 import_passwords 处理器 - 导入密码
    rpc_server.register_handler("import_passwords", |params| {
        let json_data = params.get("data")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 data 参数"))?;

        let imported_data: PasswordData = serde_json::from_str(json_data)?;

        // 加载现有数据
        let mut data = load_data()?;

        // 合并数据(跳过重复的 ID)
        for entry in imported_data.entries {
            if !data.entries.iter().any(|e| e.id == entry.id) {
                data.entries.push(entry);
            }
        }

        // 保存数据
        save_data(&data)?;

        Ok(serde_json::json!({ "success": true }))
    });

    // 注册 init 处理器
    rpc_server.register_handler("init", |_params| {
        Ok(serde_json::json!({"success": true}))
    });

    // 注册 destroy 处理器
    rpc_server.register_handler("destroy", |_params| {
        Ok(serde_json::json!({"success": true}))
    });

    // 注册 heartbeat 处理器
    rpc_server.register_handler("heartbeat", |_params| {
        Ok(serde_json::json!({"alive": true}))
    });

    // 从 stdin 读取请求
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    for line in BufReader::new(stdin).lines() {
        match line {
            Ok(req_str) => {
                let response = rpc_server.handle(&req_str);
                writeln!(stdout, "{}", response)?;
                stdout.flush()?;
            }
            Err(e) => {
                eprintln!("Error reading stdin: {}", e);
                break;
            }
        }
    }

    Ok(())
}
