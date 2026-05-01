# 插件安装使用说明

## 插件列表（8 个）

### 1. 密码管理器 (password-manager)
- **文件**: `password-manager/password-manager.wtplugin.zip`
- **功能**: AES-256-GCM 加密存储密码，导入/导出/搜索/过滤，URL 快速打开，剪贴板复制
- **权限**: filesystem, clipboard

### 2. 双因素验证 (auth)
- **文件**: `auth-plugin/auth.wtplugin.zip`
- **功能**: TOTP 动态验证码 (支持 Google Authenticator)，6/8 位验证码，自动刷新，二维码导入
- **权限**: clipboard

### 3. JSON 工具 (json-tools)
- **文件**: `json-tools/json-tools.wtplugin.zip`
- **功能**: JSON 格式化/压缩/转义，树形可视化编辑，节点选择和删除，实时语法验证

### 4. 文本比对 (text-diff)
- **文件**: `text-diff/text-diff.wtplugin.zip`
- **功能**: Monaco Editor 并排比对，字符级差异高亮与统计
- **权限**: filesystem, clipboard

### 5. 数据库文档 (db-doc)
- **文件**: `db-doc/db-doc.wtplugin.zip`
- **功能**: 连接 MySQL/PostgreSQL，生成表结构文档 (Markdown/Word)，步骤导航，表搜索与过滤
- **权限**: filesystem, network

### 6. K8s IP转发 (k8s-forward)
- **文件**: `k8s-forward/k8s-forward.wtplugin.zip`
- **功能**: Kuboard DEX SSO 发现 Pod，SSH 隧道 + HTTP 代理转发，3 Tab 前端
- **权限**: filesystem, network

### 7. 数据库路由 (db-router)
- **文件**: `db-router/db-router.wtplugin.zip`
- **功能**: Rhai 脚本引擎解析数据库和表路由，丰富内置函数，双栏布局
- **权限**: filesystem

### 8. 对象存储 (object-storage)
- **文件**: `object-storage/object-storage.wtplugin.zip`
- **功能**: 阿里云 OSS + 腾讯云 COS，文件浏览/上传/下载/搜索/删除
- **权限**: network, filesystem

---

## 安装步骤

1. **启动 Work Tools 应用**
   ```bash
   cd tauri-app
   npm run tauri dev
   ```

2. **打开插件商店**
   - 点击应用底部工具栏的 🧩 按钮

3. **导入插件包**
   - 点击"导入插件"按钮
   - 选择对应的 `.wtplugin.zip` 文件
   - 插件自动安装到 `~/.worktools/plugins/`

4. **验证安装**
   - 插件自动加载，左侧边栏显示新安装的插件
   - 点击插件图标即可使用

---

## 插件包内容

每个 `.wtplugin.zip` 包含:
- `manifest.json` - 插件元数据 (id, name, version, icon, permissions 等)
- `lib<name>.dylib/.so/.dll` - Rust 动态库 (按平台)
- `assets/` - 前端资源
  - `index.html` - HTML 入口
  - `main.js` - JavaScript 代码
  - `styles.css` - 样式文件

---

## 数据存储位置

```
~/.worktools/
├── plugins/                    # 已安装的插件
│   └── <plugin-id>/
│       ├── manifest.json
│       ├── lib<name>.dylib
│       └── assets/
├── history/plugins/            # 插件持久化数据
│   ├── password-manager.json
│   ├── auth.json
│   ├── db-doc.json
│   └── ...
├── config/
│   └── installed-plugins.json  # 插件注册表
└── logs/                       # 日志文件 (按天滚动)
```

---

## 构建插件

修改插件后重新打包:

```bash
# 编译所有插件并打包
bash scripts/build-plugins.sh

# 单独编译某个插件
cargo build --release -p <plugin-name>
```

---

## 问题排查

如果插件无法加载:

1. 检查浏览器控制台是否有错误
2. 确认动态库文件名与 manifest.json 中一致
3. 检查 `~/.worktools/plugins/<plugin-id>/` 目录结构
4. 查看应用日志: `~/.worktools/logs/`
5. 验证动态库导出符号 (macOS):
   ```bash
   nm -gU ~/.worktools/plugins/<name>/lib<name>.dylib | grep plugin_create
   ```

---

**Work Tools Platform** - 基于 Tauri 2.x + Rust 的可扩展工具平台
