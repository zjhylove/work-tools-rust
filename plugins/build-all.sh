#!/bin/bash

# 插件快速构建和打包脚本

set -e  # 遇到错误立即退出

# 获取脚本所在目录的绝对路径
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "🚀 开始插件构建流程..."
echo "工作目录: $SCRIPT_DIR"

# 1. 构建前端资源
echo ""
echo "📦 构建插件前端..."
cd "$SCRIPT_DIR/password-manager/frontend"
npm run build
echo "✓ password-manager 前端构建完成"

cd "$SCRIPT_DIR/auth-plugin/frontend"
npm run build
echo "✓ auth-plugin 前端构建完成"

cd "$SCRIPT_DIR/json-tools/frontend"
npm run build
echo "✓ json-tools 前端构建完成"

cd "$SCRIPT_DIR/text-diff/frontend"
npm run build
echo "✓ text-diff 前端构建完成"

# 2. 编译 Rust 动态库
echo ""
echo "🔨 编译 Rust 动态库..."
cd "$SCRIPT_DIR/password-manager"
cargo build --release
echo "✓ password-manager 动态库编译完成"

cd "$SCRIPT_DIR/auth-plugin"
cargo build --release
echo "✓ auth-plugin 动态库编译完成"

cd "$SCRIPT_DIR/json-tools"
cargo build --release
echo "✓ json-tools 动态库编译完成"

cd "$SCRIPT_DIR/text-diff"
cargo build --release
echo "✓ text-diff 动态库编译完成"

# 3. 打包插件
echo ""
echo "📦 打包插件..."

# 定义插件列表
PLUGINS=(
    "password-manager:password_manager"
    "auth:auth_plugin"
    "json-tools:json_tools"
    "text-diff:text_diff"
)

for PLUGIN_INFO in "${PLUGINS[@]}"; do
    IFS=':' read -r PLUGIN_ID DLL_NAME <<< "$PLUGIN_INFO"

    case "$PLUGIN_ID" in
        "password-manager") PLUGIN_DIR="password-manager" ;;
        "auth") PLUGIN_DIR="auth-plugin" ;;
        "json-tools") PLUGIN_DIR="json-tools" ;;
        "text-diff") PLUGIN_DIR="text-diff" ;;
    esac

    OUTPUT_FILE="$SCRIPT_DIR/$PLUGIN_DIR/$PLUGIN_ID.wtplugin.zip"
    DLL_PATH="$SCRIPT_DIR/$PLUGIN_DIR/target/release/lib${DLL_NAME}.dylib"
    SO_PATH="$SCRIPT_DIR/$PLUGIN_DIR/target/release/lib${DLL_NAME}.so"
    MANIFEST_PATH="$SCRIPT_DIR/$PLUGIN_DIR/manifest.json"
    ASSETS_PATH="$SCRIPT_DIR/$PLUGIN_DIR/assets"

    echo "  打包 $PLUGIN_ID..."

    # 删除旧的 zip 文件
    rm -f "$OUTPUT_FILE"

    # 根据平台选择动态库
    if [[ "$OSTYPE" == "darwin"* ]]; then
        LIB_FILE="$DLL_PATH"
    elif [[ "$OSTYPE" == "msys"* || "$OSTYPE" == "win32"* ]]; then
        LIB_FILE="$SCRIPT_DIR/$PLUGIN_DIR/target/release/${DLL_NAME}.dll"
    else
        LIB_FILE="$SO_PATH"
    fi

    # 创建 zip 文件
    pushd "$SCRIPT_DIR/$PLUGIN_DIR" > /dev/null
    zip -r "$OUTPUT_FILE" manifest.json assets -j "$LIB_FILE" 2>/dev/null
    popd > /dev/null

    echo "    ✓ $PLUGIN_ID.wtplugin.zip"
done

echo ""
echo "✨ 所有插件构建和打包完成!"
echo ""
echo "插件包位置:"
for PLUGIN_INFO in "${PLUGINS[@]}"; do
    IFS=':' read -r PLUGIN_ID _ <<< "$PLUGIN_INFO"
    case "$PLUGIN_ID" in
        "password-manager") PLUGIN_DIR="password-manager" ;;
        "auth") PLUGIN_DIR="auth-plugin" ;;
        "json-tools") PLUGIN_DIR="json-tools" ;;
        "text-diff") PLUGIN_DIR="text-diff" ;;
    esac
    echo "  - $SCRIPT_DIR/$PLUGIN_DIR/$PLUGIN_ID.wtplugin.zip"
done
