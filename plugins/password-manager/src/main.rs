use anyhow::Result;
use std::io::{BufRead, BufReader, Write};
use worktools_shared_types::{PluginInfo, UiField, ViewSchema};
use worktools_rpc_protocol::RpcServer;

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

/// UI Schema
fn get_view_schema() -> ViewSchema {
    ViewSchema {
        fields: vec![
            UiField::Input {
                label: "服务名称".to_string(),
                key: "service".to_string(),
                placeholder: Some("例如: Google".to_string()),
                default: None,
            },
            UiField::Input {
                label: "用户名".to_string(),
                key: "username".to_string(),
                placeholder: Some("输入用户名或邮箱".to_string()),
                default: None,
            },
            UiField::Input {
                label: "密码".to_string(),
                key: "password".to_string(),
                placeholder: Some("输入密码".to_string()),
                default: None,
            },
            UiField::Button {
                label: "保存密码".to_string(),
                key: "save".to_string(),
                action: "save_password".to_string(),
            },
        ],
    }
}

fn main() -> Result<()> {
    // 配置日志输出到 stderr,保持 stdout 纯净用于 JSON-RPC
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stderr)
        .init();

    let mut rpc_server = RpcServer::new();

    // 注册 RPC 方法
    rpc_server.register_handler("get_info", |_params| {
        let info = get_plugin_info();
        Ok(serde_json::to_value(info)?)
    });

    rpc_server.register_handler("get_view", |_params| {
        let schema = get_view_schema();
        Ok(serde_json::to_value(schema)?)
    });

    rpc_server.register_handler("init", |_params| {
        // 初始化插件
        Ok(serde_json::json!({"success": true}))
    });

    rpc_server.register_handler("destroy", |_params| {
        // 清理资源
        Ok(serde_json::json!({"success": true}))
    });

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
