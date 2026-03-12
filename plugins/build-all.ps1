# Plugin quick build and package script (PowerShell)

$ErrorActionPreference = "Stop"

# Get script directory
$SCRIPT_DIR = Split-Path -Parent $MyInvocation.MyCommand.Path

Write-Host "Starting plugin build process..." -ForegroundColor Cyan
Write-Host "Working directory: $SCRIPT_DIR"

# 1. Build frontend assets
Write-Host ""
Write-Host "Building plugin frontends..." -ForegroundColor Yellow

Push-Location (Join-Path $SCRIPT_DIR "password-manager\frontend")
npm run build
Pop-Location
Write-Host "  password-manager frontend build complete" -ForegroundColor Green

Push-Location (Join-Path $SCRIPT_DIR "auth-plugin\frontend")
npm run build
Pop-Location
Write-Host "  auth-plugin frontend build complete" -ForegroundColor Green

Push-Location (Join-Path $SCRIPT_DIR "json-tools\frontend")
npm run build
Pop-Location
Write-Host "  json-tools frontend build complete" -ForegroundColor Green

Push-Location (Join-Path $SCRIPT_DIR "text-diff\frontend")
npm run build
Pop-Location
Write-Host "  text-diff frontend build complete" -ForegroundColor Green

# 2. Compile Rust dynamic libraries
Write-Host ""
Write-Host "Compiling Rust dynamic libraries..." -ForegroundColor Yellow

Push-Location (Join-Path $SCRIPT_DIR "password-manager")
cargo build --release
Pop-Location
Write-Host "  password-manager dynamic library compile complete" -ForegroundColor Green

Push-Location (Join-Path $SCRIPT_DIR "auth-plugin")
cargo build --release
Pop-Location
Write-Host "  auth-plugin dynamic library compile complete" -ForegroundColor Green

Push-Location (Join-Path $SCRIPT_DIR "json-tools")
cargo build --release
Pop-Location
Write-Host "  json-tools dynamic library compile complete" -ForegroundColor Green

Push-Location (Join-Path $SCRIPT_DIR "text-diff")
cargo build --release
Pop-Location
Write-Host "  text-diff dynamic library compile complete" -ForegroundColor Green

# 3. Package plugins
Write-Host ""
Write-Host "Packaging plugins..." -ForegroundColor Yellow

$plugins = @(
    @{ id = "password-manager"; dir = "password-manager"; dll = "password_manager.dll" },
    @{ id = "auth"; dir = "auth-plugin"; dll = "auth_plugin.dll" },
    @{ id = "json-tools"; dir = "json-tools"; dll = "json_tools.dll" },
    @{ id = "text-diff"; dir = "text-diff"; dll = "text_diff.dll" }
)

foreach ($plugin in $plugins) {
    $pluginDir = Join-Path $SCRIPT_DIR $plugin.dir
    $outputFile = Join-Path $pluginDir "$($plugin.id).wtplugin.zip"
    $dllPath = Join-Path $pluginDir "target\release\$($plugin.dll)"
    $manifestPath = Join-Path $pluginDir "manifest.json"
    $assetsPath = Join-Path $pluginDir "assets"

    Write-Host "  Packaging $($plugin.id)..."

    # Remove old zip if exists
    if (Test-Path $outputFile) { Remove-Item $outputFile -Force }

    # Create zip using Compress-Archive
    $filesToZip = @()
    if (Test-Path $manifestPath) { $filesToZip += $manifestPath }
    if (Test-Path $dllPath) { $filesToZip += $dllPath }
    if (Test-Path $assetsPath) { $filesToZip += $assetsPath }

    if ($filesToZip.Count -gt 0) {
        Compress-Archive -Path $filesToZip -DestinationPath $outputFile -CompressionLevel Optimal
        Write-Host "    ✓ $($plugin.id).wtplugin.zip" -ForegroundColor Green
    } else {
        Write-Host "    ⚠ Skipping $($plugin.id) - no files found" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "All plugins built and packaged successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "Plugin package locations:"
foreach ($plugin in $plugins) {
    $zipPath = Join-Path $SCRIPT_DIR "$($plugin.dir)\$($plugin.id).wtplugin.zip"
    Write-Host "  - $zipPath"
}
