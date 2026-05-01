# Work Tools 构建脚本

## 脚本列表

### 1. check-env.sh / check-env.ps1

检查开发环境是否配置正确:
- Rust 工具链 (rustc, cargo)
- Node.js 和 npm
- Tauri CLI
- 平台特定的依赖 (macOS, Windows, Linux)

```bash
bash scripts/check-env.sh    # macOS/Linux
.\scripts\check-env.ps1      # Windows PowerShell
```

### 2. build-plugins.sh / build-plugins.ps1

**自动发现模式** — 扫描 `plugins/` 目录，自动构建和打包所有插件为 `.wtplugin.zip`。

构建流程:
1. 检查构建环境 (cargo, zip)
2. 编译所有 Rust 动态库 (`cargo build --release`)
3. 扫描 plugins 目录，对每个插件:
   - 构建前端 (如存在)
   - 提取动态库文件名
   - 打包 manifest.json + 动态库 + assets/ → .wtplugin.zip
4. 显示构建统计

```bash
bash scripts/build-plugins.sh    # macOS/Linux
.\scripts\build-plugins.ps1      # Windows PowerShell
```

输出产物:
- `plugins/password-manager/password-manager.wtplugin.zip`
- `plugins/auth-plugin/auth.wtplugin.zip`
- `plugins/json-tools/json-tools.wtplugin.zip`
- `plugins/text-diff/text-diff.wtplugin.zip`
- `plugins/db-doc/db-doc.wtplugin.zip`
- `plugins/k8s-forward/k8s-forward.wtplugin.zip`
- `plugins/db-router/db-router.wtplugin.zip`
- `plugins/object-storage/object-storage.wtplugin.zip`

---

## 手动构建单个插件

```bash
# 编译动态库
cargo build --release -p <plugin-name>

# 进入插件目录，构建前端 (如有)
cd plugins/<plugin-name>/frontend && npm install && npm run build

# 打包 (以 macOS 为例)
cd .. && cp ../../target/release/lib<name>.dylib .
zip -r <plugin-id>.wtplugin.zip manifest.json lib<name>.dylib assets/
rm lib<name>.dylib
```

---

## 跨平台构建

| 平台 | 动态库扩展 | 示例 |
|------|-----------|------|
| macOS | `.dylib` | `libpassword_manager.dylib` |
| Linux | `.so` | `libpassword_manager.so` |
| Windows | `.dll` | `password_manager.dll` |

---

## 故障排除

### 1. cargo build 失败
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. npm run build 失败
```bash
cd plugins/<plugin-name>/frontend && npm install
```

### 3. zip 命令未找到
```bash
# macOS
brew install zip
# Linux
sudo apt-get install zip
```

---

## 插件包结构

```
<plugin-id>.wtplugin.zip
├── manifest.json          # 插件元数据
├── lib<name>.dylib        # 动态库 (macOS)
├── lib<name>.so           # 动态库 (Linux)
├── <name>.dll             # 动态库 (Windows)
└── assets/                # 前端资源
    ├── index.html
    ├── main.js
    └── styles.css
```

## 验证插件包

```bash
unzip -l <plugin-id>.wtplugin.zip
```
