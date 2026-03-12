# 插件安装包使用说明

## 插件包列表

### 1. Auth Plugin (双因素认证)
- **文件**: `auth-plugin/auth.wtplugin.zip`
- **大小**: 325 KB
- **版本**: 1.0.0
- **功能**: TOTP 双因素认证,支持 Google Authenticator 等

### 2. Password Manager Plugin (密码管理器)
- **文件**: `password-manager/password-manager.wtplugin.zip`
- **大小**: 339 KB
- **版本**: 1.0.0
- **功能**: 本地安全存储和管理密码

## 安装步骤

1. **启动 Work Tools 应用**
   ```bash
   cd tauri-app
   npm run tauri dev
   ```

2. **打开插件商店**
   - 点击应用右下角的 🧩 按钮

3. **导入插件包**
   - 点击 "选择文件" 按钮
   - 选择对应的 `.wtplugin.zip` 文件
   - 点击 "安装插件"

4. **验证安装**
   - 插件会自动加载
   - 左侧边栏会显示新安装的插件

## 插件包内容

每个插件包包含:
- `manifest.json` - 插件元数据
- `lib<name>.dylib` - Rust 动态库 (macOS)
- `assets/` - 前端资源目录
  - `index.html` - HTML 入口
  - `main.js` - JavaScript 代码
  - `styles.css` - 样式文件

## 数据存储位置

插件数据存储在: `~/.worktools/history/plugins/`
- `auth.json` - 双因素认证数据
- `password-manager.json` - 密码管理器数据

## 重新构建插件

如果需要修改插件并重新打包:

```bash
# 进入插件目录
cd plugins

# 执行构建脚本
bash build-all.sh

# 插件包会自动生成在各自插件目录下
```

## 技术细节

- **前端框架**: React 19 + Vite
- **后端**: Rust (cdylib)
- **通信**: Tauri IPC
- **样式**: CSS Variables (主题可配置)
- **打包格式**: ZIP (.wtplugin.zip)

## 问题排查

如果插件无法加载:

1. 检查浏览器控制台是否有错误
2. 确认动态库文件名正确
3. 检查 `~/.worktools/plugins/` 目录结构
4. 查看应用日志

---

**生成时间**: 2026-03-02 23:40
**工作目录**: /Users/zj/Project/Rust/work-tools-rust
