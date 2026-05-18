# Work Tools 用户指南

## 下载安装

从 [GitHub Releases](https://github.com/user/work-tools-rust/releases) 页面下载对应平台的安装包。

| 平台 | 安装包 | 说明 |
|------|--------|------|
| macOS (Apple Silicon) | `Work-Tools_*_aarch64.dmg` | M1/M2/M3/M4 芯片 |
| macOS (Intel) | `Work-Tools_*_x64.dmg` | Intel 芯片 |
| Windows | `Work-Tools_*_x64_en-US.msi` | 64 位系统 |
| Linux | `work-tools_*_amd64.deb` 或 `.AppImage` | x64 发行版 |

### macOS 安装

1. 双击 `.dmg` 文件
2. 将 Work Tools 图标拖入 Applications 文件夹
3. 首次打开时，右键点击应用 -> "打开"，在弹出对话框中确认

> 首次启动可能被 macOS Gatekeeper 拦截。前往"系统设置" -> "隐私与安全性" -> 点击"仍要打开"。

### Windows 安装

1. 双击 `.msi` 文件
2. 按照安装向导操作
3. 如果弹出 SmartScreen 提示，点击"更多信息" -> "仍要运行"

### Linux 安装

```bash
# Debian/Ubuntu (.deb)
sudo dpkg -i work-tools_1.0.0_amd64.deb

# AppImage (无需安装)
chmod +x work-tools_1.0.0_amd64.AppImage
./work-tools_1.0.0_amd64.AppImage
```

## 首次启动

启动应用后，你会看到：

- **侧边栏**：左侧显示已安装的插件列表，每个插件有图标和名称
- **主内容区**：右侧显示当前选中插件的界面
- **侧边栏底部**：主题切换按钮（浅色/暗色）

应用启动时会自动加载 `~/.worktools/plugins/` 下的所有已安装插件。

## 插件管理

### 导入插件

1. 获取 `.wtplugin.zip` 插件包文件
2. 点击侧边栏顶部的插件市场按钮
3. 选择 `.wtplugin.zip` 文件进行导入
4. 导入成功后，插件会出现在侧边栏中

插件包格式说明：

```
.wtplugin.zip
├── manifest.json              # 插件元数据
├── lib<name>.dylib/.so/.dll   # 动态库（按平台）
└── assets/                    # 前端资源
    ├── index.html
    ├── main.js
    └── styles.css
```

### 卸载插件

1. 在侧边栏中找到要卸载的插件
2. 右键点击或进入插件管理界面
3. 确认卸载
4. 插件文件和数据将被清除

## 主题切换

应用支持浅色和暗色两种主题：

- 点击侧边栏底部的 moon/sun 图标按钮进行切换
- 主题设置会自动保存到 localStorage，下次启动时保持上次的选择
- 所有插件界面跟随主题自动切换

## 日志查看

日志系统提供三层输出：

| 层级 | 输出位置 | 用途 |
|------|----------|------|
| 控制台 | stdout | 开发调试，带 ANSI 颜色 |
| 文件 | `~/.worktools/logs/` | 持久化，按天滚动 |
| 内存 | 日志环形缓冲 (1000 条) | 前端查询 |

### 开发者工具

应用内置日志查看功能，可通过前端日志面板查看最近的操作日志，支持按 level、plugin、时间范围过滤。

## 数据存储位置

所有应用数据存储在 `~/.worktools/` 目录下：

```
~/.worktools/
├── plugins/                     # 已安装插件
│   └── <plugin-id>/            # 每个插件一个目录
│       ├── lib<name>.dylib     # 动态库
│       └── assets/             # 前端资源
├── config/                      # 配置文件
│   └── installed-plugins.json  # 插件注册表
├── logs/                        # 日志文件（按天滚动）
└── history/                     # 历史数据
    └── plugins/                 # 插件持久化数据
        └── <plugin-id>.json    # 每个插件的数据文件
```

| 平台 | 实际路径 |
|------|----------|
| macOS | `/Users/<用户名>/.worktools/` |
| Windows | `C:\Users\<用户名>\.worktools\` |
| Linux | `/home/<用户名>/.worktools/` |

## 常见问题 FAQ

### Q: macOS 提示"应用已损坏，无法打开"

这是 macOS Gatekeeper 的安全机制。在终端执行：

```bash
xattr -cr /Applications/Work\ Tools.app
```

然后重新打开应用。

### Q: 插件导入后没有显示在侧边栏

- 确认 `.wtplugin.zip` 文件格式正确，包含 `manifest.json`
- 确认插件的平台动态库与当前系统匹配（macOS 需要 `.dylib`，Windows 需要 `.dll`，Linux 需要 `.so`）
- 查看 `~/.worktools/logs/` 下的日志文件，搜索 "加载" 或 "失败" 相关信息

### Q: 如何清理所有应用数据

删除整个 `~/.worktools/` 目录即可：

```bash
rm -rf ~/.worktools
```

注意：此操作不可逆，会删除所有插件数据（包括密码管理器中保存的密码）。

### Q: 插件数据存储在哪里

插件数据存储在 `~/.worktools/history/plugins/` 目录下，每个插件有独立的 JSON 文件。数据采用原子写入（先写临时文件再 rename），防止写入过程中程序崩溃导致数据损坏。

### Q: 如何获取插件包

从 GitHub Releases 页面下载 `plugins-<platform>.zip`，解压后得到各插件的 `.wtplugin.zip` 文件。也可从开发者处获取单独的插件包。

### Q: 应用启动慢

首次启动需要加载所有已安装插件的动态库，后续启动会更快。如果安装了大量插件，启动时间会相应增加。

### Q: 如何查看错误日志

日志文件存储在 `~/.worktools/logs/` 目录下，按日期命名。可使用文本编辑器或命令行工具查看：

```bash
# 查看今天的日志
cat ~/.worktools/logs/$(date +%Y-%m-%d).log

# 搜索错误
grep "ERROR" ~/.worktools/logs/*.log
```
