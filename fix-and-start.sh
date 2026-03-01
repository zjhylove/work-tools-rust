#!/bin/bash

echo "🔍 诊断和修复脚本"
echo "=================="

cd /Users/zj/Project/Rust/work-tools-rust

echo ""
echo "1️⃣ 杀死所有相关进程..."
killall node 2>/dev/null
killall tauri 2>/dev/null
sleep 2

echo ""
echo "2️⃣ 清理所有缓存..."
rm -rf tauri-app/dist
rm -rf tauri-app/target/.tauri-dev
rm -rf tauri-app/node_modules/.vite
rm -rf tauri-app/.tauri
rm -rf target/debug
rm -rf target/release

echo ""
echo "3️⃣ 清理 Tauri 本地数据..."
rm -rf ~/Library/Application\ Support/com.tauri.app
rm -rf ~/Library/Caches/com.tauri.app

echo ""
echo "4️⃣ 重新构建插件..."
cargo build --release -p password-manager -p auth-plugin

echo ""
echo "5️⃣ 安装插件到用户目录..."
mkdir -p ~/.worktools/plugins/password-manager
mkdir -p ~/.worktools/plugins/auth-plugin
cp target/release/password-manager ~/.worktools/plugins/password-manager/
cp target/release/auth-plugin ~/.worktools/plugins/auth-plugin/

echo ""
echo "6️⃣ 验证插件..."
echo "测试 password-manager:"
echo '{"jsonrpc":"2.0","method":"get_info","params":{},"id":1}' | ~/.worktools/plugins/password-manager/password-manager 2>/dev/null | head -1

echo ""
echo "测试 auth-plugin:"
echo '{"jsonrpc":"2.0","method":"get_info","params":{},"id":1}' | ~/.worktools/plugins/auth-plugin/auth-plugin 2>/dev/null | head -1

echo ""
echo "7️⃣ 清理前端构建..."
cd tauri-app
rm -rf node_modules/.vite dist

echo ""
echo "8️⃣ 启动开发服务器..."
echo "请等待服务器启动..."
npm run tauri dev
