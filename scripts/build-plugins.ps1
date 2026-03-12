<<<<<<< HEAD
﻿# 插件打包脚本 (PowerShell)
# 用于构建并打包密码管理器、双因素验证和 JSON 工具插件

# 设置控制台编码为 UTF-8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

$ErrorActionPreference = "Stop"

# 项目根目录
$PROJECT_ROOT = Split-Path -Parent $PSScriptRoot
=======
# 插件打包脚本 (PowerShell)
# 用于构建并打包密码管理器、双因素验证和 JSON 工具插件

$ErrorActionPreference = "Stop"

# 项目根目录
$PROJECT_ROOT = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
>>>>>>> 713fb380a703968981c6d5fef029cd3ecbb99b18
$PLUGINS_DIR = Join-Path $PROJECT_ROOT "plugins"
$TARGET_DIR = Join-Path $PROJECT_ROOT "target\release"

Write-Host "========================================" -ForegroundColor Blue
Write-Host "  Work Tools 插件打包脚本" -ForegroundColor Blue
Write-Host "========================================" -ForegroundColor Blue
Write-Host ""

# 检查环境
Write-Host "[1/6] 检查构建环境..." -ForegroundColor Yellow
try {
    $null = Get-Command cargo -ErrorAction Stop
    $null = Get-Command zip -ErrorAction Stop
    Write-Host "✓ 构建环境检查通过" -ForegroundColor Green
} catch {
    Write-Host "✗ 错误: 未找到必要的命令" -ForegroundColor Red
    exit 1
}
Write-Host ""

# 编译 Rust 动态库
Write-Host "[2/6] 编译 Rust 动态库..." -ForegroundColor Yellow
Push-Location $PROJECT_ROOT
cargo build --release
Pop-Location
Write-Host "✓ 动态库编译完成" -ForegroundColor Green
Write-Host ""

# 构建密码管理器插件
Write-Host "[3/6] 构建密码管理器插件..." -ForegroundColor Yellow
$PASSWORD_MANAGER_DIR = Join-Path $PLUGINS_DIR "password-manager"
$PASSWORD_MANAGER_FRONTEND = Join-Path $PASSWORD_MANAGER_DIR "frontend"

if (Test-Path $PASSWORD_MANAGER_FRONTEND) {
    Write-Host "  → 构建密码管理器前端..."
    Push-Location $PASSWORD_MANAGER_FRONTEND
<<<<<<< HEAD

    # 检查并安装依赖
    if (-not (Test-Path "node_modules")) {
        Write-Host "    安装依赖..." -ForegroundColor Gray
        npm install | Out-Null
    }

=======
>>>>>>> 713fb380a703968981c6d5fef029cd3ecbb99b18
    npm run build | Out-Null
    Pop-Location
    Write-Host "  ✓ 前端构建完成" -ForegroundColor Green

    Write-Host "  → 打包密码管理器插件..."
    Push-Location $PASSWORD_MANAGER_DIR

    # 删除旧的包
    Remove-Item -Force "password-manager.wtplugin.zip" -ErrorAction SilentlyContinue

    # 复制动态库 (Windows 使用 .dll)
    $LIB_NAME = "password_manager.dll"
    Copy-Item (Join-Path $TARGET_DIR $LIB_NAME) .

    # 打包
    zip -r password-manager.wtplugin.zip manifest.json $LIB_NAME assets/ | Out-Null

    # 清理临时文件
    Remove-Item -Force $LIB_NAME

    # 显示包信息
    $PACKAGE_SIZE = (Get-Item password-manager.wtplugin.zip).Length / 1KB
    Write-Host "  ✓ 打包完成: password-manager.wtplugin.zip ($([math]::Round($PACKAGE_SIZE, 2)) KB)" -ForegroundColor Green

    Pop-Location
} else {
    Write-Host "  ⚠ 密码管理器前端目录不存在,跳过" -ForegroundColor Yellow
}
Write-Host ""

# 构建双因素验证插件
Write-Host "[4/6] 构建双因素验证插件..." -ForegroundColor Yellow
$AUTH_PLUGIN_DIR = Join-Path $PLUGINS_DIR "auth-plugin"
$AUTH_PLUGIN_FRONTEND = Join-Path $AUTH_PLUGIN_DIR "frontend"

if (Test-Path $AUTH_PLUGIN_FRONTEND) {
    Write-Host "  → 构建双因素验证前端..."
    Push-Location $AUTH_PLUGIN_FRONTEND
<<<<<<< HEAD

    # 检查并安装依赖
    if (-not (Test-Path "node_modules")) {
        Write-Host "    安装依赖..." -ForegroundColor Gray
        npm install | Out-Null
    }

=======
>>>>>>> 713fb380a703968981c6d5fef029cd3ecbb99b18
    npm run build | Out-Null
    Pop-Location
    Write-Host "  ✓ 前端构建完成" -ForegroundColor Green

    Write-Host "  → 打包双因素验证插件..."
    Push-Location $AUTH_PLUGIN_DIR

    # 删除旧的包
    Remove-Item -Force "auth.wtplugin.zip" -ErrorAction SilentlyContinue

    # 复制动态库 (Windows 使用 .dll)
    $LIB_NAME = "auth_plugin.dll"
    Copy-Item (Join-Path $TARGET_DIR $LIB_NAME) .

    # 打包
    zip -r auth.wtplugin.zip manifest.json $LIB_NAME assets/ | Out-Null

    # 清理临时文件
    Remove-Item -Force $LIB_NAME

    # 显示包信息
    $PACKAGE_SIZE = (Get-Item auth.wtplugin.zip).Length / 1KB
    Write-Host "  ✓ 打包完成: auth.wtplugin.zip ($([math]::Round($PACKAGE_SIZE, 2)) KB)" -ForegroundColor Green

    Pop-Location
} else {
    Write-Host "  ⚠ 双因素验证前端目录不存在,跳过" -ForegroundColor Yellow
}
Write-Host ""

# 构建 JSON 工具插件
Write-Host "[5/6] 构建 JSON 工具插件..." -ForegroundColor Yellow
$JSON_TOOLS_DIR = Join-Path $PLUGINS_DIR "json-tools"
$JSON_TOOLS_FRONTEND = Join-Path $JSON_TOOLS_DIR "frontend"

if (Test-Path $JSON_TOOLS_FRONTEND) {
    Write-Host "  → 构建 JSON 工具前端..."
    Push-Location $JSON_TOOLS_FRONTEND
<<<<<<< HEAD

    # 检查并安装依赖
    if (-not (Test-Path "node_modules")) {
        Write-Host "    安装依赖..." -ForegroundColor Gray
        npm install | Out-Null
    }

=======
>>>>>>> 713fb380a703968981c6d5fef029cd3ecbb99b18
    npm run build | Out-Null
    Pop-Location
    Write-Host "  ✓ 前端构建完成" -ForegroundColor Green

    Write-Host "  → 打包 JSON 工具插件..."
    Push-Location $JSON_TOOLS_DIR

    # 删除旧的包
    Remove-Item -Force "json-tools.wtplugin.zip" -ErrorAction SilentlyContinue

    # 复制动态库 (Windows 使用 .dll)
    $LIB_NAME = "json_tools.dll"
    Copy-Item (Join-Path $TARGET_DIR $LIB_NAME) .

    # 打包
    zip -r json-tools.wtplugin.zip manifest.json $LIB_NAME assets/ | Out-Null

    # 清理临时文件
    Remove-Item -Force $LIB_NAME

    # 显示包信息
    $PACKAGE_SIZE = (Get-Item json-tools.wtplugin.zip).Length / 1KB
    Write-Host "  ✓ 打包完成: json-tools.wtplugin.zip ($([math]::Round($PACKAGE_SIZE, 2)) KB)" -ForegroundColor Green

    Pop-Location
} else {
    Write-Host "  ⚠ JSON 工具前端目录不存在,跳过" -ForegroundColor Yellow
}
Write-Host ""

# 显示打包结果
Write-Host "[6/6] 打包结果汇总" -ForegroundColor Yellow
Write-Host "========================================" -ForegroundColor Blue

$PASSWORD_MANAGER_ZIP = Join-Path $PASSWORD_MANAGER_DIR "password-manager.wtplugin.zip"
if (Test-Path $PASSWORD_MANAGER_ZIP) {
    Write-Host "✓ $PASSWORD_MANAGER_ZIP" -ForegroundColor Green
}

$AUTH_PLUGIN_ZIP = Join-Path $AUTH_PLUGIN_DIR "auth.wtplugin.zip"
if (Test-Path $AUTH_PLUGIN_ZIP) {
    Write-Host "✓ $AUTH_PLUGIN_ZIP" -ForegroundColor Green
}

$JSON_TOOLS_ZIP = Join-Path $JSON_TOOLS_DIR "json-tools.wtplugin.zip"
if (Test-Path $JSON_TOOLS_ZIP) {
    Write-Host "✓ $JSON_TOOLS_ZIP" -ForegroundColor Green
}

Write-Host "========================================" -ForegroundColor Blue
Write-Host ""
Write-Host "🎉 所有插件打包完成!" -ForegroundColor Green
Write-Host ""
Write-Host "你可以通过以下方式安装插件:"
Write-Host "  1. 启动应用"
Write-Host "  2. 点击插件市场按钮 (🧩)"
Write-Host "  3. 选择对应的 .wtplugin.zip 文件导入"
Write-Host ""
