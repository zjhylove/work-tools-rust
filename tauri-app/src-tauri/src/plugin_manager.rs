use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};
use worktools_shared_types::PluginInfo;

/// 插件进程信息
#[derive(Debug)]
pub struct PluginProcess {
    pub info: PluginInfo,
    pub child: Option<Child>,
    pub is_installed: bool,
}

/// 插件管理器
pub struct PluginManager {
    plugins: RwLock<HashMap<String, PluginProcess>>,
    plugin_dir: PathBuf,
}

impl PluginManager {
    /// 创建新的插件管理器
    pub fn new() -> Result<Self> {
        let user_dirs = directories::UserDirs::new()
            .ok_or_else(|| anyhow::anyhow!("无法找到用户主目录"))?;
        let plugin_dir = user_dirs.home_dir().join(".worktools/plugins");

        // 创建插件目录
        std::fs::create_dir_all(&plugin_dir)
            .context("创建插件目录失败")?;

        Ok(Self {
            plugins: RwLock::new(HashMap::new()),
            plugin_dir,
        })
    }

    /// 初始化插件管理器,扫描可用插件
    pub async fn init(&self) -> Result<()> {
        tracing::info!("初始化插件管理器,插件目录: {:?}", self.plugin_dir);

        // 扫描插件目录
        let entries = std::fs::read_dir(&self.plugin_dir)
            .context("读取插件目录失败")?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // 查找可执行文件
            if path.is_dir() {
                if let Some(plugin_name) = path.file_name().and_then(|n| n.to_str()) {
                    let exe_path = path.join(plugin_name);
                    if exe_path.exists() {
                        self.discover_plugin(&exe_path).await?;
                    }
                }
            }
        }

        tracing::info!("插件管理器初始化完成,发现 {} 个插件", self.plugins.read().await.len());
        Ok(())
    }

    /// 发现并注册插件
    async fn discover_plugin(&self, exe_path: &Path) -> Result<()> {
        let plugin_name = exe_path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        tracing::info!("发现插件: {}", plugin_name);

        // 启动插件获取信息
        let info = self.get_plugin_info(exe_path).await?;

        let mut plugins = self.plugins.write().await;
        plugins.insert(
            info.id.clone(),
            PluginProcess {
                info,
                child: None,
                is_installed: false,
            },
        );

        Ok(())
    }

    /// 获取插件信息
    async fn get_plugin_info(&self, exe_path: &Path) -> Result<PluginInfo> {
        // 启动插件进程
        let mut child = Command::new(exe_path)
            .arg("--mode")
            .arg("info")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("启动插件进程失败")?;

        // 发送 get_info 请求
        let request = r#"{"jsonrpc":"2.0","method":"get_info","params":{},"id":1}"#;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(request.as_bytes()).await?;
            stdin.flush().await?;
        }

        // 读取响应
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            if let Some(line_result) = lines.next_line().await? {
                let response: Value = serde_json::from_str(&line_result)?;
                if let Some(_result) = response.get("result") {
                    let info: PluginInfo = serde_json::from_value(_result.clone())?;
                    child.kill().await.ok();
                    return Ok(info);
                }
            }
        }

        child.kill().await.ok();
        anyhow::bail!("获取插件信息失败")
    }

    /// 安装插件
    pub async fn install_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;

        if let Some(plugin) = plugins.get_mut(plugin_id) {
            if plugin.is_installed {
                tracing::warn!("插件已安装: {}", plugin_id);
                return Ok(());
            }

            // TODO: 启动插件进程
            plugin.is_installed = true;
            tracing::info!("插件安装成功: {}", plugin_id);
        }

        Ok(())
    }

    /// 卸载插件
    pub async fn uninstall_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;

        if let Some(plugin) = plugins.get_mut(plugin_id) {
            // TODO: 停止插件进程
            plugin.is_installed = false;
            tracing::info!("插件卸载成功: {}", plugin_id);
        }

        Ok(())
    }

    /// 获取所有可用插件
    pub async fn get_available_plugins(&self) -> Vec<PluginInfo> {
        self.plugins
            .read()
            .await
            .values()
            .filter(|p| !p.is_installed)
            .map(|p| p.info.clone())
            .collect()
    }

    /// 获取所有已安装插件
    pub async fn get_installed_plugins(&self) -> Vec<PluginInfo> {
        self.plugins
            .read()
            .await
            .values()
            .filter(|p| p.is_installed)
            .map(|p| p.info.clone())
            .collect()
    }

    /// 根据 ID 获取插件信息
    pub async fn get_plugin(&self, plugin_id: &str) -> Option<PluginInfo> {
        self.plugins
            .read()
            .await
            .get(plugin_id)
            .map(|p| p.info.clone())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new().expect("无法创建插件管理器")
    }
}
