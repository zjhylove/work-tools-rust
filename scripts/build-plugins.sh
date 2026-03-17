#!/bin/bash

# 插件打包脚本 (自动发现模式)
# 自动扫描 plugins 目录下的所有插件并打包

set -e  # 遇到错误立即退出

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 项目根目录
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PLUGINS_DIR="${PROJECT_ROOT}/plugins"
TARGET_DIR="${PROJECT_ROOT}/target/release"

# 检测当前平台
detect_platform() {
    case "$(uname -s)" in
        Darwin)
            echo "macos"
            ;;
        Linux)
            echo "linux"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            echo "windows"
            ;;
        *)
            echo "unknown"
            ;;
    esac
}

PLATFORM=$(detect_platform)

# 获取动态库文件名
get_lib_name() {
    local manifest_file="$1"
    local platform="$2"

    # 从 manifest.json 读取文件名
    if [ "$platform" = "macos" ]; then
        grep -A 3 '"files"' "$manifest_file" | grep '"macos"' | sed 's/.*: *"\([^"]*\)".*/\1/'
    elif [ "$platform" = "linux" ]; then
        grep -A 4 '"files"' "$manifest_file" | grep '"linux"' | sed 's/.*: *"\([^"]*\)".*/\1/'
    elif [ "$platform" = "windows" ]; then
        grep -A 5 '"files"' "$manifest_file" | grep '"windows"' | sed 's/.*: *"\([^"]*\)".*/\1/'
    fi
}

# 构建单个插件
build_plugin() {
    local plugin_dir="$1"
    local plugin_name="$(basename "$plugin_dir")"
    local manifest_file="${plugin_dir}/manifest.json"
    local frontend_dir="${plugin_dir}/frontend"

    # 检查 manifest.json 是否存在
    if [ ! -f "$manifest_file" ]; then
        echo -e "${YELLOW}  ⚠ ${plugin_name}: 缺少 manifest.json,跳过${NC}"
        return 0
    fi

    # 读取插件信息
    local plugin_id=$(grep -o '"id"[[:space:]]*:[[:space:]]*"[^"]*"' "$manifest_file" | sed 's/.*: *"\([^"]*\)".*/\1/')
    local package_name="${plugin_id}.wtplugin.zip"

    echo -e "${CYAN}→ 构建插件: ${plugin_name} (${plugin_id})${NC}"

    # 构建前端 (如果存在)
    if [ -d "$frontend_dir" ]; then
        echo "  → 构建前端..."
        cd "$frontend_dir"
        if npm run build > /dev/null 2>&1; then
            echo -e "${GREEN}  ✓ 前端构建完成${NC}"
        else
            echo -e "${RED}  ✗ 前端构建失败${NC}"
            return 1
        fi
    else
        echo -e "${YELLOW}  ⚠ 前端目录不存在,跳过前端构建${NC}"
    fi

    # 获取动态库名称
    local lib_name=$(get_lib_name "$manifest_file" "$PLATFORM")

    if [ -z "$lib_name" ]; then
        echo -e "${RED}  ✗ 无法从 manifest.json 读取动态库配置${NC}"
        return 1
    fi

    # 打包插件
    echo "  → 打包插件..."
    cd "$plugin_dir"

    # 删除旧的包
    rm -f "$package_name"

    # 复制动态库
    if [ ! -f "${TARGET_DIR}/${lib_name}" ]; then
        echo -e "${RED}  ✗ 动态库不存在: ${TARGET_DIR}/${lib_name}${NC}"
        echo -e "${YELLOW}  提示: 请先运行 'cargo build --release' 编译所有插件${NC}"
        return 1
    fi

    cp "${TARGET_DIR}/${lib_name}" .

    # 打包
    zip -r "$package_name" \
        manifest.json \
        "$lib_name" \
        assets/ > /dev/null 2>&1 || true

    # 清理临时文件
    rm -f "$lib_name"

    # 显示包信息
    if [ -f "$package_name" ]; then
        PACKAGE_SIZE=$(du -h "$package_name" | cut -f1)
        echo -e "${GREEN}  ✓ 打包完成: ${package_name} (${PACKAGE_SIZE})${NC}"
    else
        echo -e "${RED}  ✗ 打包失败${NC}"
        return 1
    fi

    echo ""
}

# 主函数
main() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}  Work Tools 插件打包脚本${NC}"
    echo -e "${BLUE}  平台: ${PLATFORM}${NC}"
    echo -e "${BLUE}========================================${NC}"
    echo ""

    # 检查环境
    echo -e "${YELLOW}[1/4] 检查构建环境...${NC}"
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
    echo -e "${YELLOW}[2/4] 编译 Rust 动态库...${NC}"
    cd "${PROJECT_ROOT}"
    cargo build --release
    echo -e "${GREEN}✓ 动态库编译完成${NC}"
    echo ""

    # 扫描并构建所有插件
    echo -e "${YELLOW}[3/4] 扫描并构建插件...${NC}"
    echo ""

    # 统计变量
    local total_count=0
    local success_count=0
    local failed_count=0
    local skipped_count=0

    # 遍历 plugins 目录
    for plugin_dir in "${PLUGINS_DIR}"/*; do
        # 跳过非目录文件
        if [ ! -d "$plugin_dir" ]; then
            continue
        fi

        # 跳过隐藏目录
        local plugin_name="$(basename "$plugin_dir")"
        if [[ "$plugin_name" == .* ]]; then
            continue
        fi

        ((total_count++))

        # 构建插件
        if build_plugin "$plugin_dir"; then
            ((success_count++))
        else
            ((failed_count++))
        fi
    done

    # 显示构建统计
    echo -e "${YELLOW}[4/4] 构建统计${NC}"
    echo -e "${BLUE}========================================${NC}"
    echo -e "总插件数: ${CYAN}${total_count}${NC}"
    echo -e "${GREEN}成功: ${success_count}${NC}"
    if [ $failed_count -gt 0 ]; then
        echo -e "${RED}失败: ${failed_count}${NC}"
    fi
    echo -e "${BLUE}========================================${NC}"
    echo ""

    # 显示打包结果
    echo -e "${YELLOW}插件包位置:${NC}"
    for plugin_dir in "${PLUGINS_DIR}"/*; do
        if [ -d "$plugin_dir" ]; then
            local package_name="$(basename "$plugin_dir").wtplugin.zip"
            local package_path="${plugin_dir}/${package_name}"
            if [ -f "$package_path" ]; then
                echo -e "${GREEN}✓${NC} ${package_path}"
            fi
        fi
    done
    echo ""

    # 显示安装提示
    if [ $success_count -gt 0 ]; then
        echo -e "${GREEN}🎉 插件打包完成!${NC}"
        echo ""
        echo "你可以通过以下方式安装插件:"
        echo "  1. 启动应用"
        echo "  2. 点击插件市场按钮 (🧩)"
        echo "  3. 选择对应的 .wtplugin.zip 文件导入"
        echo ""
    fi

    # 如果有失败的插件,返回错误码
    if [ $failed_count -gt 0 ]; then
        exit 1
    fi
}

# 执行主函数
main
