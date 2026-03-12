# Plugin packaging script (PowerShell)
# Build and package password-manager, auth-plugin, json-tools and text-diff plugins

# Set console encoding to UTF-8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

$ErrorActionPreference = "Stop"

# Project root directory
$PROJECT_ROOT = Split-Path -Parent $PSScriptRoot
$PLUGINS_DIR = Join-Path $PROJECT_ROOT "plugins"
$TARGET_DIR = Join-Path $PROJECT_ROOT "target\release"

Write-Host "========================================" -ForegroundColor Blue
Write-Host "  Work Tools Plugin Build Script" -ForegroundColor Blue
Write-Host "========================================" -ForegroundColor Blue
Write-Host ""

# Check environment
Write-Host "[1/7] Checking build environment..." -ForegroundColor Yellow
try {
    $null = Get-Command cargo -ErrorAction Stop
    $null = Get-Command zip -ErrorAction Stop
    Write-Host "  Build environment check passed" -ForegroundColor Green
} catch {
    Write-Host "  Error: Required commands not found" -ForegroundColor Red
    exit 1
}
Write-Host ""

# Compile Rust dynamic libraries
Write-Host "[2/7] Compiling Rust dynamic libraries..." -ForegroundColor Yellow
Push-Location $PROJECT_ROOT
cargo build --release
Pop-Location
Write-Host "  Dynamic library compilation complete" -ForegroundColor Green
Write-Host ""

# Build password-manager plugin
Write-Host "[3/7] Building password-manager plugin..." -ForegroundColor Yellow
$PASSWORD_MANAGER_DIR = Join-Path $PLUGINS_DIR "password-manager"
$PASSWORD_MANAGER_FRONTEND = Join-Path $PASSWORD_MANAGER_DIR "frontend"

if (Test-Path $PASSWORD_MANAGER_FRONTEND) {
    Write-Host "  Building password-manager frontend..."
    Push-Location $PASSWORD_MANAGER_FRONTEND
    npm run build | Out-Null
    Pop-Location
    Write-Host "  Frontend build complete" -ForegroundColor Green

    Write-Host "  Packaging password-manager plugin..."
    Push-Location $PASSWORD_MANAGER_DIR

    # Remove old package
    Remove-Item -Force "password-manager.wtplugin.zip" -ErrorAction SilentlyContinue

    # Copy dynamic library (Windows uses .dll)
    $LIB_NAME = "password_manager.dll"
    Copy-Item (Join-Path $TARGET_DIR $LIB_NAME) .

    # Package
    zip -r password-manager.wtplugin.zip manifest.json $LIB_NAME assets/ | Out-Null

    # Clean up temp files
    Remove-Item -Force $LIB_NAME

    # Show package info
    $PACKAGE_SIZE = (Get-Item password-manager.wtplugin.zip).Length / 1KB
    Write-Host "  Package complete: password-manager.wtplugin.zip ($([math]::Round($PACKAGE_SIZE, 2)) KB)" -ForegroundColor Green

    Pop-Location
} else {
    Write-Host "  Password-manager frontend directory not found, skipping" -ForegroundColor Yellow
}
Write-Host ""

# Build auth-plugin
Write-Host "[4/7] Building auth-plugin..." -ForegroundColor Yellow
$AUTH_PLUGIN_DIR = Join-Path $PLUGINS_DIR "auth-plugin"
$AUTH_PLUGIN_FRONTEND = Join-Path $AUTH_PLUGIN_DIR "frontend"

if (Test-Path $AUTH_PLUGIN_FRONTEND) {
    Write-Host "  Building auth-plugin frontend..."
    Push-Location $AUTH_PLUGIN_FRONTEND
    npm run build | Out-Null
    Pop-Location
    Write-Host "  Frontend build complete" -ForegroundColor Green

    Write-Host "  Packaging auth-plugin..."
    Push-Location $AUTH_PLUGIN_DIR

    # Remove old package
    Remove-Item -Force "auth.wtplugin.zip" -ErrorAction SilentlyContinue

    # Copy dynamic library (Windows uses .dll)
    $LIB_NAME = "auth_plugin.dll"
    Copy-Item (Join-Path $TARGET_DIR $LIB_NAME) .

    # Package
    zip -r auth.wtplugin.zip manifest.json $LIB_NAME assets/ | Out-Null

    # Clean up temp files
    Remove-Item -Force $LIB_NAME

    # Show package info
    $PACKAGE_SIZE = (Get-Item auth.wtplugin.zip).Length / 1KB
    Write-Host "  Package complete: auth.wtplugin.zip ($([math]::Round($PACKAGE_SIZE, 2)) KB)" -ForegroundColor Green

    Pop-Location
} else {
    Write-Host "  Auth-plugin frontend directory not found, skipping" -ForegroundColor Yellow
}
Write-Host ""

# Build json-tools plugin
Write-Host "[5/7] Building json-tools plugin..." -ForegroundColor Yellow
$JSON_TOOLS_DIR = Join-Path $PLUGINS_DIR "json-tools"
$JSON_TOOLS_FRONTEND = Join-Path $JSON_TOOLS_DIR "frontend"

if (Test-Path $JSON_TOOLS_FRONTEND) {
    Write-Host "  Building json-tools frontend..."
    Push-Location $JSON_TOOLS_FRONTEND
    npm run build | Out-Null
    Pop-Location
    Write-Host "  Frontend build complete" -ForegroundColor Green

    Write-Host "  Packaging json-tools plugin..."
    Push-Location $JSON_TOOLS_DIR

    # Remove old package
    Remove-Item -Force "json-tools.wtplugin.zip" -ErrorAction SilentlyContinue

    # Copy dynamic library (Windows uses .dll)
    $LIB_NAME = "json_tools.dll"
    Copy-Item (Join-Path $TARGET_DIR $LIB_NAME) .

    # Package
    zip -r json-tools.wtplugin.zip manifest.json $LIB_NAME assets/ | Out-Null

    # Clean up temp files
    Remove-Item -Force $LIB_NAME

    # Show package info
    $PACKAGE_SIZE = (Get-Item json-tools.wtplugin.zip).Length / 1KB
    Write-Host "  Package complete: json-tools.wtplugin.zip ($([math]::Round($PACKAGE_SIZE, 2)) KB)" -ForegroundColor Green

    Pop-Location
} else {
    Write-Host "  JSON-tools frontend directory not found, skipping" -ForegroundColor Yellow
}
Write-Host ""

# Build text-diff plugin
Write-Host "[6/7] Building text-diff plugin..." -ForegroundColor Yellow
$TEXT_DIFF_DIR = Join-Path $PLUGINS_DIR "text-diff"
$TEXT_DIFF_FRONTEND = Join-Path $TEXT_DIFF_DIR "frontend"

if (Test-Path $TEXT_DIFF_FRONTEND) {
    Write-Host "  Building text-diff frontend..."
    Push-Location $TEXT_DIFF_FRONTEND
    npm run build | Out-Null
    Pop-Location
    Write-Host "  Frontend build complete" -ForegroundColor Green

    Write-Host "  Packaging text-diff plugin..."
    Push-Location $TEXT_DIFF_DIR

    # Remove old package
    Remove-Item -Force "text-diff.wtplugin.zip" -ErrorAction SilentlyContinue

    # Copy dynamic library (Windows uses .dll)
    $LIB_NAME = "text_diff.dll"
    Copy-Item (Join-Path $TARGET_DIR $LIB_NAME) .

    # Package
    zip -r text-diff.wtplugin.zip manifest.json $LIB_NAME assets/ | Out-Null

    # Clean up temp files
    Remove-Item -Force $LIB_NAME

    # Show package info
    $PACKAGE_SIZE = (Get-Item text-diff.wtplugin.zip).Length / 1KB
    Write-Host "  Package complete: text-diff.wtplugin.zip ($([math]::Round($PACKAGE_SIZE, 2)) KB)" -ForegroundColor Green

    Pop-Location
} else {
    Write-Host "  Text-diff frontend directory not found, skipping" -ForegroundColor Yellow
}
Write-Host ""

# Show packaging results
Write-Host "[7/7] Package Summary" -ForegroundColor Yellow
Write-Host "========================================" -ForegroundColor Blue

$PASSWORD_MANAGER_ZIP = Join-Path $PASSWORD_MANAGER_DIR "password-manager.wtplugin.zip"
if (Test-Path $PASSWORD_MANAGER_ZIP) {
    Write-Host "  OK $PASSWORD_MANAGER_ZIP" -ForegroundColor Green
}

$AUTH_PLUGIN_ZIP = Join-Path $AUTH_PLUGIN_DIR "auth.wtplugin.zip"
if (Test-Path $AUTH_PLUGIN_ZIP) {
    Write-Host "  OK $AUTH_PLUGIN_ZIP" -ForegroundColor Green
}

$JSON_TOOLS_ZIP = Join-Path $JSON_TOOLS_DIR "json-tools.wtplugin.zip"
if (Test-Path $JSON_TOOLS_ZIP) {
    Write-Host "  OK $JSON_TOOLS_ZIP" -ForegroundColor Green
}

$TEXT_DIFF_ZIP = Join-Path $TEXT_DIFF_DIR "text-diff.wtplugin.zip"
if (Test-Path $TEXT_DIFF_ZIP) {
    Write-Host "  OK $TEXT_DIFF_ZIP" -ForegroundColor Green
}

Write-Host "========================================" -ForegroundColor Blue
Write-Host ""
Write-Host "All plugins packaged successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "Install plugins via:"
Write-Host "  1. Start the application"
Write-Host "  2. Click plugin market button"
Write-Host "  3. Select .wtplugin.zip file to import"
Write-Host ""
