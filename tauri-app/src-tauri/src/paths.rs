use anyhow::Result;
use std::path::PathBuf;

fn worktools_base() -> Result<PathBuf> {
    let user_dirs =
        directories::UserDirs::new().ok_or_else(|| anyhow::anyhow!("无法找到用户主目录"))?;
    Ok(user_dirs.home_dir().join(".worktools"))
}

pub fn plugins_dir() -> Result<PathBuf> {
    Ok(worktools_base()?.join("plugins"))
}

pub fn config_dir() -> Result<PathBuf> {
    Ok(worktools_base()?.join("config"))
}

pub fn logs_dir() -> Result<PathBuf> {
    Ok(worktools_base()?.join("logs"))
}

pub fn history_dir() -> Result<PathBuf> {
    Ok(worktools_base()?.join("history"))
}
