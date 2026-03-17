# 插件构建脚本使用说明

## 概述

本项目的插件构建系统已升级为**自动发现模式**,可以自动扫描 `plugins/` 目录下的所有插件并执行构建和打包。

**核心优势**:
- ✅ **零维护成本**: 添加新插件时无需修改构建脚本
- ✅ **自动识别**: 自动读取 `manifest.json` 获取插件配置
- ✅ **跨平台支持**: 自动检测当前平台并使用正确的动态库格式
- ✅ **构建统计**: 显示构建成功/失败的插件数量

## 构建脚本

### 1. 主构建脚本 (推荐)

#### Linux/macOS
```bash
bash scripts/build-plugins.sh
```

#### Windows PowerShell
```powershell
.\scripts\build-plugins.ps1
```

**功能**:
1. 检查构建环境 (cargo, zip)
2. 编译所有 Rust 动态库
3. 自动扫描 `plugins/` 目录
4. 读取每个插件的 `manifest.json`
5. 构建前端 (如果有 `frontend/` 目录)
6. 打包生成 `.wtplugin.zip` 文件

**输出示例**:
```
========================================
  Work Tools 插件打包脚本
  平台: macos
========================================

[1/4] 检查构建环境...
✓ 构建环境检查通过

[2/4] 编译 Rust 动态库...
✓ 动态库编译完成

[3/4] 扫描并构建插件...

→ 构建插件: auth-plugin (auth)
  → 构建前端...
  ✓ 前端构建完成
  → 打包插件...
  ✓ 打包完成: auth.wtplugin.zip (368K)

→ 构建插件: json-tools (json-tools)
  ✓ 打包完成: json-tools.wtplugin.zip (276K)

→ 构建插件: password-manager (password-manager)
  ✓ 打包完成: password-manager.wtplugin.zip (392K)

→ 构建插件: text-diff (text-diff)
  ✓ 打包完成: text-diff.wtplugin.zip (3.4M)

[4/4] 构建统计
========================================
总插件数: 4
成功: 4
========================================
```

### 2. 快速构建脚本

```bash
bash plugins/build-all.sh
```

**功能**:
- 先构建所有插件的前端
- 编译 Rust 动态库
- 调用主构建脚本打包

## 添加新插件

### 步骤

1. **创建插件目录**:
```bash
mkdir plugins/my-new-plugin
```

2. **创建 `manifest.json`**:
```json
{
  "id": "my-new-plugin",
  "name": "我的新插件",
  "description": "插件描述",
  "version": "1.0.0",
  "icon": "🔧",
  "author": "Your Name",
  "homepage": "https://github.com/your/repo",
  "files": {
    "macos": "libmy_new_plugin.dylib",
    "linux": "libmy_new_plugin.so",
    "windows": "my_new_plugin.dll"
  },
  "assets": {
    "entry": "index.html"
  }
}
```

3. **运行构建脚本**:
```bash
bash scripts/build-plugins.sh
```

**就是这么简单!** 脚本会自动:
- 发现你的新插件
- 读取 `manifest.json` 配置
- 构建 frontend (如果存在)
- 打包生成 `my-new-plugin.wtplugin.zip`

### 无需修改构建脚本

与旧版本不同,新版本的构建脚本会自动扫描 `plugins/` 目录,因此**不需要手动更新任何构建脚本**。

## 插件目录结构规范

```
plugins/
├── my-new-plugin/              # 插件目录
│   ├── Cargo.toml             # Rust 配置
│   ├── manifest.json          # 插件元数据 (必需)
│   ├── src/                   # Rust 源码
│   ├── frontend/              # 前端目录 (可选)
│   │   ├── package.json
│   │   └── ...
│   └── assets/                # 静态资源 (由前端构建生成)
│       └── index.html
└── ...
```

## 常见问题

### 1. 前端构建失败

**问题**: `✗ 前端构建失败`

**解决方案**:
```bash
# 进入插件前端目录
cd plugins/my-plugin/frontend

# 安装依赖
npm install

# 单独测试构建
npm run build
```

### 2. 动态库编译失败

**问题**: `✗ 动态库不存在: target/release/libxxx.dylib`

**解决方案**:
```bash
# 单独编译插件
cd plugins/my-plugin
cargo build --release
```

### 3. manifest.json 格式错误

**问题**: `无法从 manifest.json 读取动态库配置`

**解决方案**: 检查 JSON 格式是否正确:
```bash
# 验证 JSON 格式
cat plugins/my-plugin/manifest.json | jq .
```

### 4. 跳过某个插件的构建

**方法**: 临时重命名插件目录:
```bash
# 添加前缀点 (.) 使其被脚本忽略
mv plugins/my-plugin plugins/.my-plugin
```

## 技术实现

### 自动发现机制

**Bash 版本**:
```bash
for plugin_dir in "${PLUGINS_DIR}"/*; do
    # 跳过非目录文件和隐藏目录
    if [ ! -d "$plugin_dir" ]; then
        continue
    fi

    plugin_name="$(basename "$plugin_dir")"
    if [[ "$plugin_name" == .* ]]; then
        continue
    fi

    # 构建插件
    build_plugin "$plugin_dir"
done
```

**PowerShell 版本**:
```powershell
$pluginDirs = Get-ChildItem -Path $PLUGINS_DIR -Directory |
    Where-Object { $_.Name -notlike '.*' }

foreach ($pluginDir in $pluginDirs) {
    Build-Plugin -PluginDir $pluginDir.FullName
}
```

### 动态库名称读取

脚本从 `manifest.json` 读取平台特定的动态库文件名:

```json
{
  "files": {
    "macos": "libtext_diff.dylib",
    "linux": "libtext_diff.so",
    "windows": "text_diff.dll"
  }
}
```

**实现**:
```bash
get_lib_name() {
    local manifest_file="$1"
    local platform="$2"

    if [ "$platform" = "macos" ]; then
        grep -A 3 '"files"' "$manifest_file" |
        grep '"macos"' |
        sed 's/.*: *"\([^"]*\)".*/\1/'
    fi
    # ... 其他平台
}
```

## 贡献指南

如果你需要为构建系统添加新功能:

1. 保持跨平台兼容 (Linux/macOS/Windows)
2. 遵循现有的错误处理模式
3. 添加清晰的日志输出
4. 更新本文档

## 相关文档

- [插件开发规范](../CLAUDE.md#插件开发规范)
- [项目架构](../CLAUDE.md#架构关键概念)
- [构建脚本源码](./build-plugins.sh)
