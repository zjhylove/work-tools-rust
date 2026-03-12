# 快速参考: Work Tools 插件打包

## 一键打包所有插件

```bash
# macOS/Linux
./scripts/build-plugins.sh

# Windows PowerShell
.\scripts\build-plugins.ps1
```

## 脚本功能

✅ 自动编译 Rust 动态库
✅ 自动构建前端资源
✅ 自动打包为 .wtplugin.zip
✅ 彩色输出,进度清晰
✅ 错误处理,失败即停

## 输出位置

打包完成的插件位于:
- `plugins/password-manager/password-manager.wtplugin.zip`
- `plugins/auth-plugin/auth.wtplugin.zip`

## 安装方法

1. 启动应用
2. 点击 🧩 按钮
3. 导入 .wtplugin.zip 文件

## 仅构建前端

```bash
# 密码管理器
cd plugins/password-manager/frontend
npm run build

# 双因素验证
cd plugins/auth-plugin/frontend
npm run build
```

## 仅编译动态库

```bash
cargo build --release
```

## 验证插件包

```bash
unzip -l password-manager.wtplugin.zip
```

应该看到 6 个文件:
- manifest.json
- lib*.dylib (或 .so/.dll)
- assets/
- assets/index.html
- assets/main.js
- assets/styles.css
