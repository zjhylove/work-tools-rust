#!/bin/bash
# 跨平台环境检查脚本

set -e

# 获取脚本所在目录的父目录(项目根目录)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🔍 检查开发环境..."
echo "项目根目录: $PROJECT_ROOT"
echo ""

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查函数
check_command() {
    if command -v $1 &> /dev/null; then
        echo -e "${GREEN}✓${NC} $1 已安装: $($1 --version 2>&1 | head -n 1 || echo $1)"
        return 0
    else
        echo -e "${RED}✗${NC} $1 未安装"
        return 1
    fi
}

# 检测操作系统
detect_os() {
    case "$(uname -s)" in
        Darwin)
            echo -e "${GREEN}检测到: macOS${NC}"
            OS="macos"
            ;;
        Linux)
            echo -e "${GREEN}检测到: Linux${NC}"
            OS="linux"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            echo -e "${GREEN}检测到: Windows${NC}"
            OS="windows"
            ;;
        *)
            echo -e "${RED}未知操作系统${NC}"
            exit 1
            ;;
    esac
}

detect_os

echo ""
echo "📦 核心工具:"
echo "-----------------------------------"

MISSING_COUNT=0

# 检查 Rust
check_command rustc || MISSING_COUNT=$((MISSING_COUNT + 1))
check_command cargo || MISSING_COUNT=$((MISSING_COUNT + 1))

# 检查 Node.js
check_command node || MISSING_COUNT=$((MISSING_COUNT + 1))
check_command npm || MISSING_COUNT=$((MISSING_COUNT + 1))

echo ""
echo "🔧 平台特定工具:"
echo "-----------------------------------"

if [ "$OS" = "macos" ]; then
    check_command xcode-select || MISSING_COUNT=$((MISSING_COUNT + 1))
    # 检查是否安装了 Xcode Command Line Tools
    if xcode-select -p &> /dev/null; then
        echo -e "${GREEN}✓${NC} Xcode Command Line Tools 已安装"
    else
        echo -e "${RED}✗${NC} Xcode Command Line Tools 未安装"
        echo -e "  ${YELLOW}运行: xcode-select --install${NC}"
        MISSING_COUNT=$((MISSING_COUNT + 1))
    fi

elif [ "$OS" = "linux" ]; then
    # 检查 WebKitGTK
    if pkg-config --exists webkit2gtk-4.1; then
        echo -e "${GREEN}✓${NC} libwebkit2gtk-4.1-dev 已安装"
    else
        echo -e "${RED}✗${NC} libwebkit2gtk-4.1-dev 未安装"
        echo -e "  ${YELLOW}运行: sudo apt install libwebkit2gtk-4.1-dev${NC}"
        MISSING_COUNT=$((MISSING_COUNT + 1))
    fi

elif [ "$OS" = "windows" ]; then
    # 检查 WebView2
    if reg query "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" &> /dev/null; then
        echo -e "${GREEN}✓${NC} WebView2 已安装"
    else
        echo -e "${YELLOW}⚠${NC} WebView2 可能未安装"
        echo -e "  ${YELLOW}下载: https://developer.microsoft.com/en-us/microsoft-edge/webview2/${NC}"
    fi

    # 检查 Visual Studio Build Tools
    if cl.exe &> /dev/null; then
        echo -e "${GREEN}✓${NC} MSVC 已安装"
    else
        echo -e "${YELLOW}⚠${NC} MSVC 可能未安装或未在 PATH 中"
        echo -e "  ${YELLOW}需要安装 Visual Studio C++ Build Tools${NC}"
    fi
fi

echo ""
echo "📁 项目检查:"
echo "-----------------------------------"

# 切换到项目根目录进行检查
cd "$PROJECT_ROOT"

# 检查项目结构
if [ -d "tauri-app" ]; then
    echo -e "${GREEN}✓${NC} tauri-app 目录存在"
else
    echo -e "${RED}✗${NC} tauri-app 目录不存在"
    MISSING_COUNT=$((MISSING_COUNT + 1))
fi

if [ -f "tauri-app/src-tauri/Cargo.toml" ]; then
    echo -e "${GREEN}✓${NC} Cargo.toml 存在"
else
    echo -e "${RED}✗${NC} Cargo.toml 不存在"
    MISSING_COUNT=$((MISSING_COUNT + 1))
fi

if [ -f "tauri-app/src-tauri/tauri.conf.json" ]; then
    echo -e "${GREEN}✓${NC} tauri.conf.json 存在"
else
    echo -e "${RED}✗${NC} tauri.conf.json 不存在"
    MISSING_COUNT=$((MISSING_COUNT + 1))
fi

# 检查图标文件
if [ -d "tauri-app/src-tauri/icons" ]; then
    ICON_COUNT=$(ls -1 tauri-app/src-tauri/icons/*.{png,icns,ico} 2>/dev/null | wc -l)
    if [ $ICON_COUNT -gt 0 ]; then
        echo -e "${GREEN}✓${NC} 图标文件存在 ($ICON_COUNT 个)"
    else
        echo -e "${YELLOW}⚠${NC} 图标文件可能缺失"
    fi
else
    echo -e "${RED}✗${NC} icons 目录不存在"
    MISSING_COUNT=$((MISSING_COUNT + 1))
fi

echo ""
echo "🎯 Rust Targets:"
echo "-----------------------------------"

# 检查已安装的 targets
if command -v rustup &> /dev/null; then
    INSTALLED_TARGETS=$(rustup target list --installed 2>/dev/null || echo "")

    if [ "$OS" = "macos" ]; then
        echo "当前架构: $(uname -m)"
        if echo "$INSTALLED_TARGETS" | grep -q "x86_64-apple-darwin"; then
            echo -e "${GREEN}✓${NC} x86_64-apple-darwin (Intel)"
        else
            echo -e "${YELLOW}⚠${NC} x86_64-apple-darwin 未安装"
            echo -e "  ${YELLOW}运行: rustup target add x86_64-apple-darwin${NC}"
        fi

        if echo "$INSTALLED_TARGETS" | grep -q "aarch64-apple-darwin"; then
            echo -e "${GREEN}✓${NC} aarch64-apple-darwin (Apple Silicon)"
        else
            echo -e "${YELLOW}⚠${NC} aarch64-apple-darwin 未安装"
            echo -e "  ${YELLOW}运行: rustup target add aarch64-apple-darwin${NC}"
        fi
    elif [ "$OS" = "linux" ]; then
        if echo "$INSTALLED_TARGETS" | grep -q "x86_64-unknown-linux-gnu"; then
            echo -e "${GREEN}✓${NC} x86_64-unknown-linux-gnu"
        fi
    elif [ "$OS" = "windows" ]; then
        if echo "$INSTALLED_TARGETS" | grep -q "x86_64-pc-windows-msvc"; then
            echo -e "${GREEN}✓${NC} x86_64-pc-windows-msvc"
        fi
    fi
else
    echo -e "${YELLOW}⚠${NC} rustup 未找到"
fi

echo ""
echo "📝 总结:"
echo "-----------------------------------"

if [ $MISSING_COUNT -eq 0 ]; then
    echo -e "${GREEN}✓ 所有检查通过! 可以开始构建。${NC}"
    echo ""
    echo "下一步:"
    echo "  cd $PROJECT_ROOT/tauri-app"
    echo "  npm run tauri dev    # 开发模式"
    echo "  npm run tauri build  # 生产构建"
    exit 0
else
    echo -e "${RED}✗ 发现 $MISSING_COUNT 个问题,请修复后再继续。${NC}"
    exit 1
fi
