# 插件打包脚本 (PowerShell)
# 用于构建并打包密码管理器和双因素验证插件

$ErrorActionPreference = "Stop"

# 项目根目录
$PROJECT_ROOT = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$PLUGINS_DIR = Join-Path $PROJECT_ROOT "plugins"
$TARGET_DIR = Join-Path $PROJECT_ROOT "target\release"

Write-Host "========================================" -ForegroundColor Blue
Write-Host "  Work Tools 插件打包脚本" -ForegroundColor Blue
Write-Host "========================================" -ForegroundColor Blue
Write-Host ""

# 检查环境
Write-Host "[1/5] 检查构建环境..." -ForegroundColor Yellow
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
Write-Host "[2/5] 编译 Rust 动态库..." -ForegroundColor Yellow
Push-Location $PROJECT_ROOT
cargo build --release
Pop-Location
Write-Host "✓ 动态库编译完成" -ForegroundColor Green
Write-Host ""

# 构建密码管理器插件
Write-Host "[3/5] 构建密码管理器插件..." -ForegroundColor Yellow
$PASSWORD_MANAGER_DIR = Join-Path $PLUGINS_DIR "password-manager"
$PASSWORD_MANAGER_FRONTEND = Join-Path $PASSWORD_MANAGER_DIR "frontend"

if (Test-Path $PASSWORD_MANAGER_FRONTEND) {
    Write-Host "  → 构建密码管理器前端..."
    Push-Location $PASSWORD_MANAGER_FRONTEND
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
Write-Host "[4/5] 构建双因素验证插件..." -ForegroundColor Yellow
$AUTH_PLUGIN_DIR = Join-Path $PLUGINS_DIR "auth-plugin"
$AUTH_PLUGIN_FRONTEND = Join-Path $AUTH_PLUGIN_DIR "frontend"

if (Test-Path $AUTH_PLUGIN_FRONTEND) {
    Write-Host "  → 构建双因素验证前端..."
    Push-Location $AUTH_PLUGIN_FRONTEND
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

# 显示打包结果
Write-Host "[5/5] 打包结果汇总" -ForegroundColor Yellow
Write-Host "========================================" -ForegroundColor Blue

$PASSWORD_MANAGER_ZIP = Join-Path $PASSWORD_MANAGER_DIR "password-manager.wtplugin.zip"
if (Test-Path $PASSWORD_MANAGER_ZIP) {
    Write-Host "✓ $PASSWORD_MANAGER_ZIP" -ForegroundColor Green
}

$AUTH_PLUGIN_ZIP = Join-Path $AUTH_PLUGIN_DIR "auth.wtplugin.zip"
if (Test-Path $AUTH_PLUGIN_ZIP) {
    Write-Host "✓ $AUTH_PLUGIN_ZIP" -ForegroundColor Green
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
