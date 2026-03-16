# 跨平台环境检查脚本 (Windows)

# 获取脚本所在目录的父目录(项目根目录)
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir

Write-Host "🔍 检查开发环境..." -ForegroundColor Cyan
Write-Host "项目根目录: $projectRoot" -ForegroundColor Cyan
Write-Host ""

$missingCount = 0

# 检查命令是否存在
function Test-Command {
    param($Name)

    if (Get-Command $Name -ErrorAction SilentlyContinue) {
        $version = & $Name --version 2>$null | Select-Object -First 1
        if ($version) {
            Write-Host "✓ $Name 已安装: $version" -ForegroundColor Green
        } else {
            Write-Host "✓ $Name 已安装" -ForegroundColor Green
        }
        return $true
    } else {
        Write-Host "✗ $Name 未安装" -ForegroundColor Red
        return $false
    }
}

Write-Host "📦 核心工具:"
Write-Host "-----------------------------------"

# 检查 Rust
if (-not (Test-Command "rustc")) { $missingCount++ }
if (-not (Test-Command "cargo")) { $missingCount++ }

# 检查 Node.js
if (-not (Test-Command "node")) { $missingCount++ }
if (-not (Test-Command "npm")) { $missingCount++ }

Write-Host ""
Write-Host "🔧 平台特定工具:"
Write-Host "-----------------------------------"

# 检查 WebView2
$webView2Path = "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
if (Test-Path $webView2Path) {
    Write-Host "✓ WebView2 已安装" -ForegroundColor Green
} else {
    Write-Host "⚠ WebView2 可能未安装" -ForegroundColor Yellow
    Write-Host "  下载: https://developer.microsoft.com/en-us/microsoft-edge/webview2/" -ForegroundColor Yellow
}

# 检查 MSVC (cl.exe)
$clPath = Where.exe cl.exe 2>$null
if ($clPath) {
    Write-Host "✓ MSVC 已安装 (cl.exe)" -ForegroundColor Green
} else {
    Write-Host "⚠ MSVC 可能未安装或未在 PATH 中" -ForegroundColor Yellow
    Write-Host "  需要安装 Visual Studio C++ Build Tools" -ForegroundColor Yellow
    Write-Host "  下载: https://visualstudio.microsoft.com/downloads/" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "📁 项目检查:"
Write-Host "-----------------------------------"

# 切换到项目根目录进行检查
Push-Location $projectRoot

# 检查项目结构
if (Test-Path "tauri-app") {
    Write-Host "✓ tauri-app 目录存在" -ForegroundColor Green
} else {
    Write-Host "✗ tauri-app 目录不存在" -ForegroundColor Red
    $missingCount++
}

if (Test-Path "tauri-app\src-tauri\Cargo.toml") {
    Write-Host "✓ Cargo.toml 存在" -ForegroundColor Green
} else {
    Write-Host "✗ Cargo.toml 不存在" -ForegroundColor Red
    $missingCount++
}

if (Test-Path "tauri-app\src-tauri\tauri.conf.json") {
    Write-Host "✓ tauri.conf.json 存在" -ForegroundColor Green
} else {
    Write-Host "✗ tauri.conf.json 不存在" -ForegroundColor Red
    $missingCount++
}

# 检查图标文件
if (Test-Path "tauri-app\src-tauri\icons") {
    $iconCount = (Get-ChildItem "tauri-app\src-tauri\icons" -Filter *.png -ErrorAction SilentlyContinue).Count +
                 (Get-ChildItem "tauri-app\src-tauri\icons" -Filter *.ico -ErrorAction SilentlyContinue).Count +
                 (Get-ChildItem "tauri-app\src-tauri\icons" -Filter *.icns -ErrorAction SilentlyContinue).Count

    if ($iconCount -gt 0) {
        Write-Host "✓ 图标文件存在 ($iconCount 个)" -ForegroundColor Green
    } else {
        Write-Host "⚠ 图标文件可能缺失" -ForegroundColor Yellow
    }
} else {
    Write-Host "✗ icons 目录不存在" -ForegroundColor Red
    $missingCount++
}

# 恢复原来的目录
Pop-Location

Write-Host ""
Write-Host "🎯 Rust Targets:"
Write-Host "-----------------------------------"

if (Get-Command rustup -ErrorAction SilentlyContinue) {
    $installedTargets = rustup target list --installed 2>$null

    if ($installedTargets -match "x86_64-pc-windows-msvc") {
        Write-Host "✓ x86_64-pc-windows-msvc" -ForegroundColor Green
    }
} else {
    Write-Host "⚠ rustup 未找到" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "📝 总结:"
Write-Host "-----------------------------------"

if ($missingCount -eq 0) {
    Write-Host "✓ 所有检查通过! 可以开始构建。" -ForegroundColor Green
    Write-Host ""
    Write-Host "下一步:"
    Write-Host "  cd $projectRoot\tauri-app"
    Write-Host "  npm run tauri dev    # 开发模式"
    Write-Host "  npm run tauri build  # 生产构建"
    exit 0
} else {
    Write-Host "✗ 发现 $missingCount 个问题,请修复后再继续。" -ForegroundColor Red
    exit 1
}
