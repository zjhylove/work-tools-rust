use std::sync::Arc;
use work_tools_lib::PluginManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 测试插件管理器 ===\n");

    let manager = Arc::new(PluginManager::new()?);

    println!("初始化插件管理器...");
    manager.init().await?;

    println!("\n=== 获取已安装插件 ===");
    let installed = manager.get_installed_plugins().await;
    println!("数量: {}", installed.len());
    for (i, p) in installed.iter().enumerate() {
        println!("  [{}] id={}, name={}, icon={}", i, p.id, p.name, p.icon);
    }

    Ok(())
}
