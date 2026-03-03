#!/bin/bash

# 插件打包脚本
# 用于构建并打包密码管理器和双因素验证插件

set -e  # 遇到错误立即退出

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 项目根目录
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PLUGINS_DIR="${PROJECT_ROOT}/plugins"
TARGET_DIR="${PROJECT_ROOT}/target/release"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Work Tools 插件打包脚本${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# 检查环境
echo -e "${YELLOW}[1/5] 检查构建环境...${NC}"
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}✗ 错误: 未找到 cargo${NC}"
    exit 1
fi

if ! command -v zip &> /dev/null; then
    echo -e "${RED}✗ 错误: 未找到 zip 命令${NC}"
    exit 1
fi
echo -e "${GREEN}✓ 构建环境检查通过${NC}"
echo ""

# 编译 Rust 动态库
echo -e "${YELLOW}[2/5] 编译 Rust 动态库...${NC}"
cd "${PROJECT_ROOT}"
cargo build --release
echo -e "${GREEN}✓ 动态库编译完成${NC}"
echo ""

# 构建密码管理器插件
echo -e "${YELLOW}[3/5] 构建密码管理器插件...${NC}"
PASSWORD_MANAGER_DIR="${PLUGINS_DIR}/password-manager"
PASSWORD_MANAGER_FRONTEND="${PASSWORD_MANAGER_DIR}/frontend"

if [ -d "${PASSWORD_MANAGER_FRONTEND}" ]; then
    echo "  → 构建密码管理器前端..."
    cd "${PASSWORD_MANAGER_FRONTEND}"
    npm run build > /dev/null 2>&1
    echo -e "${GREEN}  ✓ 前端构建完成${NC}"

    echo "  → 打包密码管理器插件..."
    cd "${PASSWORD_MANAGER_DIR}"

    # 删除旧的包
    rm -f password-manager.wtplugin.zip

    # 复制动态库
    cp "${TARGET_DIR}/libpassword_manager.dylib" .

    # 打包
    zip -r password-manager.wtplugin.zip \
        manifest.json \
        libpassword_manager.dylib \
        assets/ > /dev/null

    # 清理临时文件
    rm -f libpassword_manager.dylib

    # 显示包信息
    PACKAGE_SIZE=$(du -h password-manager.wtplugin.zip | cut -f1)
    echo -e "${GREEN}  ✓ 打包完成: password-manager.wtplugin.zip (${PACKAGE_SIZE})${NC}"
else
    echo -e "${YELLOW}  ⚠ 密码管理器前端目录不存在,跳过${NC}"
fi
echo ""

# 构建双因素验证插件
echo -e "${YELLOW}[4/5] 构建双因素验证插件...${NC}"
AUTH_PLUGIN_DIR="${PLUGINS_DIR}/auth-plugin"
AUTH_PLUGIN_FRONTEND="${AUTH_PLUGIN_DIR}/frontend"

if [ -d "${AUTH_PLUGIN_FRONTEND}" ]; then
    echo "  → 构建双因素验证前端..."
    cd "${AUTH_PLUGIN_FRONTEND}"
    npm run build > /dev/null 2>&1
    echo -e "${GREEN}  ✓ 前端构建完成${NC}"

    echo "  → 打包双因素验证插件..."
    cd "${AUTH_PLUGIN_DIR}"

    # 删除旧的包
    rm -f auth.wtplugin.zip

    # 复制动态库
    cp "${TARGET_DIR}/libauth_plugin.dylib" .

    # 打包
    zip -r auth.wtplugin.zip \
        manifest.json \
        libauth_plugin.dylib \
        assets/ > /dev/null

    # 清理临时文件
    rm -f libauth_plugin.dylib

    # 显示包信息
    PACKAGE_SIZE=$(du -h auth.wtplugin.zip | cut -f1)
    echo -e "${GREEN}  ✓ 打包完成: auth.wtplugin.zip (${PACKAGE_SIZE})${NC}"
else
    echo -e "${YELLOW}  ⚠ 双因素验证前端目录不存在,跳过${NC}"
fi
echo ""

# 显示打包结果
echo -e "${YELLOW}[5/5] 打包结果汇总${NC}"
echo -e "${BLUE}========================================${NC}"

if [ -f "${PASSWORD_MANAGER_DIR}/password-manager.wtplugin.zip" ]; then
    echo -e "${GREEN}✓${NC} ${PASSWORD_MANAGER_DIR}/password-manager.wtplugin.zip"
fi

if [ -f "${AUTH_PLUGIN_DIR}/auth.wtplugin.zip" ]; then
    echo -e "${GREEN}✓${NC} ${AUTH_PLUGIN_DIR}/auth.wtplugin.zip"
fi

echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "${GREEN}🎉 所有插件打包完成!${NC}"
echo ""
echo "你可以通过以下方式安装插件:"
echo "  1. 启动应用"
echo "  2. 点击插件市场按钮 (🧩)"
echo "  3. 选择对应的 .wtplugin.zip 文件导入"
echo ""
