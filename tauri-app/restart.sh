#!/bin/bash
echo "🧹 清理缓存..."
rm -rf dist target/.tauri-dev node_modules/.vite .tauri

echo "🔨 清理 Rust 构建..."
cd src-tauri
cargo clean
cd ..

echo "📦 重新构建插件..."
cd ../..
cargo build --release -p password-manager -p auth-plugin

echo "📥 安装插件..."
mkdir -p ~/.worktools/plugins/password-manager
mkdir -p ~/.worktools/plugins/auth-plugin
cp target/release/password-manager ~/.worktools/plugins/password-manager/
cp target/release/auth-plugin ~/.worktools/plugins/auth-plugin/

echo "🚀 启动开发服务器..."
cd tauri-app
npm run tauri dev
