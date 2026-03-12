<<<<<<< HEAD
# Cross-platform environment check script (Windows)

# Get the parent directory of the script directory (project root)
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir

Write-Host "Checking development environment..." -ForegroundColor Cyan
Write-Host "Project root: $projectRoot" -ForegroundColor Cyan
=======
# 跨平台环境检查脚本 (Windows)

# 获取脚本所在目录的父目录(项目根目录)
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir

Write-Host "🔍 检查开发环境..." -ForegroundColor Cyan
Write-Host "项目根目录: $projectRoot" -ForegroundColor Cyan
>>>>>>> 713fb380a703968981c6d5fef029cd3ecbb99b18
Write-Host ""

$missingCount = 0

<<<<<<< HEAD
# Function to check if a command exists
=======
# 检查命令是否存在
>>>>>>> 713fb380a703968981c6d5fef029cd3ecbb99b18
function Test-Command {
    param($Name)

    if (Get-Command $Name -ErrorAction SilentlyContinue) {
        $version = & $Name --version 2>$null | Select-Object -First 1
        if ($version) {
<<<<<<< HEAD
            Write-Host "[OK] $Name installed: $version" -ForegroundColor Green
        } else {
            Write-Host "[OK] $Name installed" -ForegroundColor Green
        }
        return $true
    } else {
        Write-Host "[MISSING] $Name not installed" -ForegroundColor Red
=======
            Write-Host "✓ $Name 已安装: $version" -ForegroundColor Green
        } else {
            Write-Host "✓ $Name 已安装" -ForegroundColor Green
        }
        return $true
    } else {
        Write-Host "✗ $Name 未安装" -ForegroundColor Red
>>>>>>> 713fb380a703968981c6d5fef029cd3ecbb99b18
        return $false
    }
}

<<<<<<< HEAD
Write-Host "Core Tools:"
Write-Host "-----------------------------------"

# Check Rust
if (-not (Test-Command "rustc")) { $missingCount++ }
if (-not (Test-Command "cargo")) { $missingCount++ }

# Check Node.js
=======
Write-Host "📦 核心工具:"
Write-Host "-----------------------------------"

# 检查 Rust
if (-not (Test-Command "rustc")) { $missingCount++ }
if (-not (Test-Command "cargo")) { $missingCount++ }

# 检查 Node.js
>>>>>>> 713fb380a703968981c6d5fef029cd3ecbb99b18
if (-not (Test-Command "node")) { $missingCount++ }
if (-not (Test-Command "npm")) { $missingCount++ }

Write-Host ""
<<<<<<< HEAD
Write-Host "Platform Specific Tools:"
Write-Host "-----------------------------------"

# Check WebView2
$webView2Path = "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
if (Test-Path $webView2Path) {
    Write-Host "[OK] WebView2 installed" -ForegroundColor Green
} else {
    Write-Host "[WARNING] WebView2 may not be installed" -ForegroundColor Yellow
    Write-Host "  Download: https://developer.microsoft.com/en-us/microsoft-edge/webview2/" -ForegroundColor Yellow
}

# Check MSVC (cl.exe)
$clPath = Where.exe cl.exe 2>$null
if ($clPath) {
    Write-Host "[OK] MSVC installed (cl.exe)" -ForegroundColor Green
} else {
    Write-Host "[WARNING] MSVC may not be installed or not in PATH" -ForegroundColor Yellow
    Write-Host "  Need to install Visual Studio C++ Build Tools" -ForegroundColor Yellow
    Write-Host "  Download: https://visualstudio.microsoft.com/downloads/" -ForegroundColor Yellow
}

# Check zip (required for plugin packaging)
if (Get-Command zip -ErrorAction SilentlyContinue) {
    Write-Host "[OK] zip installed" -ForegroundColor Green
} else {
    Write-Host "[WARNING] zip not installed (required for plugin packaging)" -ForegroundColor Yellow
    Write-Host "  Install options:" -ForegroundColor Yellow
    Write-Host "    - choco install zip" -ForegroundColor Yellow
    Write-Host "    - scoop install zip" -ForegroundColor Yellow
    Write-Host "    - Download: http://stahlworks.com/dev/index.php?tool=zipunzip" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Project Structure:"
Write-Host "-----------------------------------"

# Switch to project root for checks
Push-Location $projectRoot

# Check project structure
if (Test-Path "tauri-app") {
    Write-Host "[OK] tauri-app directory exists" -ForegroundColor Green
} else {
    Write-Host "[MISSING] tauri-app directory does not exist" -ForegroundColor Red
=======
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
>>>>>>> 713fb380a703968981c6d5fef029cd3ecbb99b18
    $missingCount++
}

if (Test-Path "tauri-app\src-tauri\Cargo.toml") {
<<<<<<< HEAD
    Write-Host "[OK] Cargo.toml exists" -ForegroundColor Green
} else {
    Write-Host "[MISSING] Cargo.toml does not exist" -ForegroundColor Red
=======
    Write-Host "✓ Cargo.toml 存在" -ForegroundColor Green
} else {
    Write-Host "✗ Cargo.toml 不存在" -ForegroundColor Red
>>>>>>> 713fb380a703968981c6d5fef029cd3ecbb99b18
    $missingCount++
}

if (Test-Path "tauri-app\src-tauri\tauri.conf.json") {
<<<<<<< HEAD
    Write-Host "[OK] tauri.conf.json exists" -ForegroundColor Green
} else {
    Write-Host "[MISSING] tauri.conf.json does not exist" -ForegroundColor Red
    $missingCount++
}

# Check icon files
=======
    Write-Host "✓ tauri.conf.json 存在" -ForegroundColor Green
} else {
    Write-Host "✗ tauri.conf.json 不存在" -ForegroundColor Red
    $missingCount++
}

# 检查图标文件
>>>>>>> 713fb380a703968981c6d5fef029cd3ecbb99b18
if (Test-Path "tauri-app\src-tauri\icons") {
    $iconCount = (Get-ChildItem "tauri-app\src-tauri\icons" -Filter *.png -ErrorAction SilentlyContinue).Count +
                 (Get-ChildItem "tauri-app\src-tauri\icons" -Filter *.ico -ErrorAction SilentlyContinue).Count +
                 (Get-ChildItem "tauri-app\src-tauri\icons" -Filter *.icns -ErrorAction SilentlyContinue).Count

    if ($iconCount -gt 0) {
<<<<<<< HEAD
        Write-Host "[OK] Icon files exist ($iconCount files)" -ForegroundColor Green
    } else {
        Write-Host "[WARNING] Icon files may be missing" -ForegroundColor Yellow
    }
} else {
    Write-Host "[MISSING] icons directory does not exist" -ForegroundColor Red
    $missingCount++
}

# Restore original directory
Pop-Location

Write-Host ""
Write-Host "Rust Targets:"
=======
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
>>>>>>> 713fb380a703968981c6d5fef029cd3ecbb99b18
Write-Host "-----------------------------------"

if (Get-Command rustup -ErrorAction SilentlyContinue) {
    $installedTargets = rustup target list --installed 2>$null

    if ($installedTargets -match "x86_64-pc-windows-msvc") {
<<<<<<< HEAD
        Write-Host "[OK] x86_64-pc-windows-msvc" -ForegroundColor Green
    }
} else {
    Write-Host "[WARNING] rustup not found" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Summary:"
Write-Host "-----------------------------------"

if ($missingCount -eq 0) {
    Write-Host "[OK] All checks passed! Ready to build." -ForegroundColor Green
    Write-Host ""
    Write-Host "Next steps:"
    Write-Host "  cd $projectRoot\tauri-app"
    Write-Host "  npm run tauri dev    # Development mode"
    Write-Host "  npm run tauri build  # Production build"
    exit 0
} else {
    Write-Host "[ERROR] Found $missingCount issue(s). Please fix before continuing." -ForegroundColor Red
=======
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
>>>>>>> 713fb380a703968981c6d5fef029cd3ecbb99b18
    exit 1
}
