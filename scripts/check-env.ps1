# Cross-platform environment check script (Windows)

# Get the parent directory of the script directory (project root)
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir

Write-Host "Checking development environment..." -ForegroundColor Cyan
Write-Host "Project root: $projectRoot" -ForegroundColor Cyan
Write-Host ""

$missingCount = 0

# Function to check if a command exists
function Test-Command {
    param($Name)

    if (Get-Command $Name -ErrorAction SilentlyContinue) {
        $version = & $Name --version 2>$null | Select-Object -First 1
        if ($version) {
            Write-Host "[OK] $Name installed: $version" -ForegroundColor Green
        } else {
            Write-Host "[OK] $Name installed" -ForegroundColor Green
        }
        return $true
    } else {
        Write-Host "[MISSING] $Name not installed" -ForegroundColor Red
        return $false
    }
}

Write-Host "Core Tools:"
Write-Host "-----------------------------------"

# Check Rust
if (-not (Test-Command "rustc")) { $missingCount++ }
if (-not (Test-Command "cargo")) { $missingCount++ }

# Check Node.js
if (-not (Test-Command "node")) { $missingCount++ }
if (-not (Test-Command "npm")) { $missingCount++ }

Write-Host ""
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
    $missingCount++
}

if (Test-Path "tauri-app\src-tauri\Cargo.toml") {
    Write-Host "[OK] Cargo.toml exists" -ForegroundColor Green
} else {
    Write-Host "[MISSING] Cargo.toml does not exist" -ForegroundColor Red
    $missingCount++
}

if (Test-Path "tauri-app\src-tauri\tauri.conf.json") {
    Write-Host "[OK] tauri.conf.json exists" -ForegroundColor Green
} else {
    Write-Host "[MISSING] tauri.conf.json does not exist" -ForegroundColor Red
    $missingCount++
}

# Check icon files
if (Test-Path "tauri-app\src-tauri\icons") {
    $iconCount = (Get-ChildItem "tauri-app\src-tauri\icons" -Filter *.png -ErrorAction SilentlyContinue).Count +
                 (Get-ChildItem "tauri-app\src-tauri\icons" -Filter *.ico -ErrorAction SilentlyContinue).Count +
                 (Get-ChildItem "tauri-app\src-tauri\icons" -Filter *.icns -ErrorAction SilentlyContinue).Count

    if ($iconCount -gt 0) {
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
Write-Host "-----------------------------------"

if (Get-Command rustup -ErrorAction SilentlyContinue) {
    $installedTargets = rustup target list --installed 2>$null

    if ($installedTargets -match "x86_64-pc-windows-msvc") {
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
    exit 1
}
