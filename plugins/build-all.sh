#!/bin/bash

# 插件快速构建和打包脚本 (自动发现模式)
# 自动扫描当前目录下的所有插件并构建

set -e  # 遇到错误立即退出

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 获取脚本所在目录的绝对路径
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Work Tools 插件快速构建脚本${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# 统计变量
TOTAL_PLUGINS=0
FRONTEND_BUILT=0
RUST_BUILT=0

# 1. 构建所有插件的前端资源
echo -e "${YELLOW}[1/3] 构建插件前端...${NC}"
echo ""

for plugin_dir in "${SCRIPT_DIR}"/*; do
    # 跳过非目录文件和隐藏目录
    if [ ! -d "$plugin_dir" ]; then
        continue
    fi

    plugin_name="$(basename "$plugin_dir")"
    if [[ "$plugin_name" == .* ]]; then
        continue
    fi

    frontend_dir="${plugin_dir}/frontend"

    # 检查是否有前端目录
    if [ -d "$frontend_dir" ]; then
        echo -e "${CYAN}→ ${plugin_name}: 构建前端...${NC}"

        if cd "$frontend_dir" && npm run build > /dev/null 2>&1; then
            echo -e "${GREEN}  ✓ ${plugin_name} 前端构建完成${NC}"
            ((FRONTEND_BUILT++))
        else
            echo -e "${RED}  ✗ ${plugin_name} 前端构建失败${NC}"
            exit 1
        fi

        ((TOTAL_PLUGINS++))
    fi
done

echo ""
if [ $FRONTEND_BUILT -gt 0 ]; then
    echo -e "${GREEN}✓ 前端构建完成 (${FRONTEND_BUILT}/${TOTAL_PLUGINS})${NC}"
else
    echo -e "${YELLOW}⚠ 未找到需要构建的前端${NC}"
fi
echo ""

# 2. 编译所有 Rust 动态库
echo -e "${YELLOW}[2/3] 编译 Rust 动态库...${NC}"
echo ""

cd "$SCRIPT_DIR/.."

# 编译整个 workspace
if cargo build --release; then
    echo -e "${GREEN}✓ 所有 Rust 动态库编译完成${NC}"
    ((RUST_BUILT++))
else
    echo -e "${RED}✗ Rust 动态库编译失败${NC}"
    exit 1
fi

echo ""

# 3. 打包插件
echo -e "${YELLOW}[3/3] 打包插件...${NC}"
echo ""

# 调用主打包脚本
if [ -f "${SCRIPT_DIR}/../scripts/build-plugins.sh" ]; then
    bash "${SCRIPT_DIR}/../scripts/build-plugins.sh"
else
    echo -e "${RED}✗ 错误: 找不到主打包脚本 ${SCRIPT_DIR}/../scripts/build-plugins.sh${NC}"
    exit 1
fi
