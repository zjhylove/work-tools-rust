#!/bin/bash

# 插件快速构建和打包脚本

set -e  # 遇到错误立即退出

echo "🚀 开始插件构建流程..."

# 1. 构建前端资源
echo ""
echo "📦 构建插件前端..."
cd "$(dirname "$0")/password-manager/frontend"
npm run build
echo "✓ password-manager 前端构建完成"

cd "$(dirname "$0")/auth-plugin/frontend"
npm run build
echo "✓ auth-plugin 前端构建完成"

# 2. 编译 Rust 动态库
echo ""
echo "🔨 编译 Rust 动态库..."
cd "$(dirname "$0")/password-manager"
cargo build --release
echo "✓ password-manager 动态库编译完成"

cd "$(dirname "$0")/auth-plugin"
cargo build --release
echo "✓ auth-plugin 动态库编译完成"

# 3. 打包插件
echo ""
echo "📦 打包插件..."
cd "$(dirname "$0")/../tauri-app/scripts"
node package-plugin-full.js

echo ""
echo "✨ 所有插件构建和打包完成!"
echo ""
echo "插件包位置:"
echo "  - plugins/password-manager/password-manager.wtplugin.zip"
echo "  - plugins/auth-plugin/auth.wtplugin.zip"
