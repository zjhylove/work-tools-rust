# Work Tools 构建脚本

这个目录包含了 Work Tools 项目的各种构建脚本。

## 脚本列表

### 1. check-env.sh / check-env.ps1
**用途**: 检查开发环境是否配置正确

**检查项目**:
- Rust 工具链 (rustc, cargo)
- Node.js 和 npm
- Tauri CLI
- 平台特定的依赖 (macOS, Windows, Linux)

**使用方法**:
```bash
# macOS/Linux
./scripts/check-env.sh

# Windows PowerShell
.\scripts\check-env.ps1
```

### 2. build-plugins.sh / build-plugins.ps1
**用途**: 构建并打包所有插件为 .wtplugin.zip 文件

**构建流程**:
1. 检查构建环境
2. 编译 Rust 动态库
3. 构建密码管理器前端
4. 打包密码管理器插件
5. 构建双因素验证前端
6. 打包双因素验证插件

**使用方法**:
```bash
# macOS/Linux
./scripts/build-plugins.sh

# Windows PowerShell
.\scripts\build-plugins.ps1
```

**输出**:
- `plugins/password-manager/password-manager.wtplugin.zip`
- `plugins/auth-plugin/auth.wtplugin.zip`

## 安装插件

打包完成后,可以通过以下方式安装:

1. 启动 Work Tools 应用
2. 点击左侧底部的插件市场按钮 (🧩)
3. 点击"导入插件"
4. 选择对应的 `.wtplugin.zip` 文件

## 手动构建单个插件

如果只需要构建某个插件,可以手动执行以下步骤:

### 密码管理器

```bash
# 1. 编译动态库
cd /path/to/work-tools-rust
cargo build --release

# 2. 构建前端
cd plugins/password-manager/frontend
npm run build

# 3. 打包
cd ..
cp ../../target/release/libpassword_manager.dylib .
zip -r password-manager.wtplugin.zip manifest.json libpassword_manager.dylib assets/
rm libpassword_manager.dylib
```

### 双因素验证

```bash
# 1. 编译动态库
cd /path/to/work-tools-rust
cargo build --release

# 2. 构建前端
cd plugins/auth-plugin/frontend
npm run build

# 3. 打包
cd ..
cp ../../target/release/libauth_plugin.dylib .
zip -r auth.wtplugin.zip manifest.json libauth_plugin.dylib assets/
rm libauth_plugin.dylib
```

## 跨平台构建

### macOS

macOS 插件使用 `.dylib` 动态库:
- `libpassword_manager.dylib`
- `libauth_plugin.dylib`

### Linux

Linux 插件使用 `.so` 动态库:
- `libpassword_manager.so`
- `libauth_plugin.so`

### Windows

Windows 插件使用 `.dll` 动态库:
- `password_manager.dll`
- `auth_plugin.dll`

## 故障排除

### 1. cargo build 失败

确保已安装 Rust 工具链:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. npm run build 失败

确保已安装 Node.js 依赖:
```bash
cd plugins/password-manager/frontend
npm install
```

### 3. zip 命令未找到

**macOS**:
```bash
brew install zip
```

**Linux** (Ubuntu/Debian):
```bash
sudo apt-get install zip
```

**Windows**:
下载并安装 [ZIP for Windows](https://sourceforge.net/projects/gnuwin32/files/zip/3.0/zip-3.0-setup.exe/)

## 插件包结构

一个正确的 `.wtplugin.zip` 文件应该包含:

```
my-plugin.wtplugin.zip
├── manifest.json          # 插件元数据
├── libmy_plugin.dylib     # 动态库 (macOS)
├── libmy_plugin.so        # 动态库 (Linux)
├── my_plugin.dll          # 动态库 (Windows)
└── assets/                # 前端资源
    ├── index.html
    ├── main.js
    └── styles.css
```

## 验证插件包

可以使用 unzip 命令验证插件包内容:

```bash
unzip -l password-manager.wtplugin.zip
```

应该看到:
- `manifest.json`
- 动态库文件 (.dylib/.so/.dll)
- `assets/` 目录
- `assets/index.html`
- `assets/main.js`
- `assets/styles.css`
