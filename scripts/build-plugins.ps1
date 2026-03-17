# 插件打包脚本 (PowerShell - 自动发现模式)
# 自动扫描 plugins 目录下的所有插件并打包

$ErrorActionPreference = "Stop"

# 项目根目录
$PROJECT_ROOT = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$PLUGINS_DIR = Join-Path $PROJECT_ROOT "plugins"
$TARGET_DIR = Join-Path $PROJECT_ROOT "target\release"

# 检测当前平台
function Get-Platform {
    if ($IsMacOS) {
        return "macos"
    } elseif ($IsLinux) {
        return "linux"
    } elseif ($IsWindows) {
        return "windows"
    } else {
        return "unknown"
    }
}

$PLATFORM = Get-Platform

# 获取动态库文件名
function Get-LibName {
    param(
        [string]$ManifestFile,
        [string]$Platform
    )

    if (-not (Test-Path $ManifestFile)) {
        return $null
    }

    $manifest = Get-Content $ManifestFile -Raw | ConvertFrom-Json

    if ($Platform -eq "macos") {
        return $manifest.files.macos
    } elseif ($Platform -eq "linux") {
        return $manifest.files.linux
    } elseif ($Platform -eq "windows") {
        return $manifest.files.windows
    }

    return $null
}

# 构建单个插件
function Build-Plugin {
    param(
        [string]$PluginDir
    )

    $pluginName = Split-Path $PluginDir -Leaf
    $manifestFile = Join-Path $PluginDir "manifest.json"
    $frontendDir = Join-Path $PluginDir "frontend"

    # 检查 manifest.json 是否存在
    if (-not (Test-Path $manifestFile)) {
        Write-Host "  ⚠ $pluginName`: 缺少 manifest.json,跳过" -ForegroundColor Yellow
        return $false
    }

    # 读取插件信息
    $manifest = Get-Content $manifestFile -Raw | ConvertFrom-Json
    $pluginId = $manifest.id
    $packageName = "$pluginId.wtplugin.zip"

    Write-Host "→ 构建插件: $pluginName ($pluginId)" -ForegroundColor Cyan

    # 构建前端 (如果存在)
    if (Test-Path $frontendDir) {
        Write-Host "  → 构建前端..."
        Push-Location $frontendDir
        try {
            npm run build | Out-Null
            Write-Host "  ✓ 前端构建完成" -ForegroundColor Green
        } catch {
            Write-Host "  ✗ 前端构建失败" -ForegroundColor Red
            Pop-Location
            return $false
        }
        Pop-Location
    } else {
        Write-Host "  ⚠ 前端目录不存在,跳过前端构建" -ForegroundColor Yellow
    }

    # 获取动态库名称
    $libName = Get-LibName -ManifestFile $manifestFile -Platform $PLATFORM

    if (-not $libName) {
        Write-Host "  ✗ 无法从 manifest.json 读取动态库配置" -ForegroundColor Red
        return $false
    }

    # 打包插件
    Write-Host "  → 打包插件..."
    Push-Location $PluginDir

    # 删除旧的包
    Remove-Item -Force $packageName -ErrorAction SilentlyContinue

    # 复制动态库
    $libPath = Join-Path $TARGET_DIR $libName
    if (-not (Test-Path $libPath)) {
        Write-Host "  ✗ 动态库不存在: $libPath" -ForegroundColor Red
        Write-Host "  提示: 请先运行 'cargo build --release' 编译所有插件" -ForegroundColor Yellow
        Pop-Location
        return $false
    }

    Copy-Item $libPath .

    # 打包
    zip -r $packageName manifest.json $libName assets/ | Out-Null

    # 清理临时文件
    Remove-Item -Force $libName

    # 显示包信息
    if (Test-Path $packageName) {
        $packageSize = (Get-Item $packageName).Length / 1KB
        Write-Host "  ✓ 打包完成: $packageName ($([math]::Round($packageSize, 2)) KB)" -ForegroundColor Green
        Pop-Location
        return $true
    } else {
        Write-Host "  ✗ 打包失败" -ForegroundColor Red
        Pop-Location
        return $false
    }
}

# 主函数
function Main {
    Write-Host "========================================" -ForegroundColor Blue
    Write-Host "  Work Tools 插件打包脚本" -ForegroundColor Blue
    Write-Host "  平台: $PLATFORM" -ForegroundColor Blue
    Write-Host "========================================" -ForegroundColor Blue
    Write-Host ""

    # 检查环境
    Write-Host "[1/4] 检查构建环境..." -ForegroundColor Yellow
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
    Write-Host "[2/4] 编译 Rust 动态库..." -ForegroundColor Yellow
    Push-Location $PROJECT_ROOT
    cargo build --release
    Pop-Location
    Write-Host "✓ 动态库编译完成" -ForegroundColor Green
    Write-Host ""

    # 扫描并构建所有插件
    Write-Host "[3/4] 扫描并构建插件..." -ForegroundColor Yellow
    Write-Host ""

    # 统计变量
    $totalCount = 0
    $successCount = 0
    $failedCount = 0

    # 遍历 plugins 目录
    $pluginDirs = Get-ChildItem -Path $PLUGINS_DIR -Directory | Where-Object { $_.Name -notlike '.*' }

    foreach ($pluginDir in $pluginDirs) {
        $totalCount++

        if (Build-Plugin -PluginDir $pluginDir.FullName) {
            $successCount++
        } else {
            $failedCount++
        }
    }

    # 显示构建统计
    Write-Host "[4/4] 构建统计" -ForegroundColor Yellow
    Write-Host "========================================" -ForegroundColor Blue
    Write-Host "总插件数: $totalCount" -ForegroundColor Cyan
    Write-Host "成功: $successCount" -ForegroundColor Green
    if ($failedCount -gt 0) {
        Write-Host "失败: $failedCount" -ForegroundColor Red
    }
    Write-Host "========================================" -ForegroundColor Blue
    Write-Host ""

    # 显示打包结果
    Write-Host "插件包位置:" -ForegroundColor Yellow
    foreach ($pluginDir in $pluginDirs) {
        $manifestFile = Join-Path $pluginDir.FullName "manifest.json"
        if (Test-Path $manifestFile) {
            $manifest = Get-Content $manifestFile -Raw | ConvertFrom-Json
            $packageName = "$($manifest.id).wtplugin.zip"
            $packagePath = Join-Path $pluginDir.FullName $packageName

            if (Test-Path $packagePath) {
                Write-Host "✓ $packagePath" -ForegroundColor Green
            }
        }
    }
    Write-Host ""

    # 显示安装提示
    if ($successCount -gt 0) {
        Write-Host "🎉 插件打包完成!" -ForegroundColor Green
        Write-Host ""
        Write-Host "你可以通过以下方式安装插件:"
        Write-Host "  1. 启动应用"
        Write-Host "  2. 点击插件市场按钮 (🧩)"
        Write-Host "  3. 选择对应的 .wtplugin.zip 文件导入"
        Write-Host ""
    }

    # 如果有失败的插件,返回错误码
    if ($failedCount -gt 0) {
        exit 1
    }
}

# 执行主函数
Main
