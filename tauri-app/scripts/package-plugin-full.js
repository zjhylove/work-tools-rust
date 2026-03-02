#!/usr/bin/env node

/**
 * 完整插件打包脚本
 *
 * 打包 manifest.json,动态库和前端资源
 */

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";
import { execSync } from "child_process";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

async function packagePlugin(pluginId, pluginDir, dylibName) {
  console.log(`\n📦 打包插件: ${pluginId}`);

  // 输出插件包到插件自己的目录
  const outputFile = path.join(pluginDir, `${pluginId}.wtplugin.zip`);

  // 临时目录用于组装文件
  const tempDir = path.join(pluginDir, `.temp-package`);
  if (fs.existsSync(tempDir)) {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
  fs.mkdirSync(tempDir, { recursive: true });

  try {
    // 1. 复制 manifest.json
    const manifestPath = path.join(pluginDir, "manifest.json");
    if (fs.existsSync(manifestPath)) {
      fs.copyFileSync(manifestPath, path.join(tempDir, "manifest.json"));
      console.log("  ✓ 添加 manifest.json");
    } else {
      throw new Error("manifest.json 不存在");
    }

    // 2. 复制动态库
    const dylibSource = path.join(__dirname, "../../target/release", dylibName);
    if (fs.existsSync(dylibSource)) {
      fs.copyFileSync(dylibSource, path.join(tempDir, dylibName));
      console.log(`  ✓ 添加 ${dylibName}`);
    } else {
      console.warn(`  ⚠ 动态库不存在: ${dylibSource}`);
    }

    // 3. 复制 assets 目录
    const assetsSource = path.join(pluginDir, "assets");
    const assetsDest = path.join(tempDir, "assets");
    if (fs.existsSync(assetsSource)) {
      copyDirectory(assetsSource, assetsDest);
      console.log("  ✓ 添加 assets 目录");
    } else {
      console.warn(`  ⚠ assets 目录不存在: ${assetsSource}`);
    }

    // 4. 使用 zip 命令打包
    const zipArgs = ["-r", outputFile, "."];
    execSync(`zip ${zipArgs.join(" ")}`, {
      cwd: tempDir,
      stdio: "inherit",
    });

    console.log(`  ✅ 打包完成: ${outputFile}`);
  } catch (error) {
    console.error(`  ❌ 打包失败: ${error.message}`);
  } finally {
    // 清理临时目录
    if (fs.existsSync(tempDir)) {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  }
}

function copyDirectory(src, dest) {
  fs.mkdirSync(dest, { recursive: true });
  const entries = fs.readdirSync(src, { withFileTypes: true });

  for (const entry of entries) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);

    if (entry.isDirectory()) {
      copyDirectory(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

async function main() {
  console.log("🚀 开始插件打包...\n");

  // 确保动态库已编译
  console.log("📝 检查动态库...");
  const releaseDir = path.join(__dirname, "../../target/release");
  const passwordDylib = path.join(releaseDir, "libpassword_manager.dylib");
  const authDylib = path.join(releaseDir, "libauth_plugin.dylib");

  if (!fs.existsSync(passwordDylib)) {
    console.error("❌ password-manager 动态库不存在,请先编译:");
    console.error("   cd plugins/password-manager && cargo build --release");
    return;
  }
  if (!fs.existsSync(authDylib)) {
    console.error("❌ auth-plugin 动态库不存在,请先编译:");
    console.error("   cd plugins/auth-plugin && cargo build --release");
    return;
  }
  console.log("  ✓ 动态库检查通过");

  // 打包插件
  await packagePlugin(
    "password-manager",
    path.join(__dirname, "../../plugins/password-manager"),
    "libpassword_manager.dylib",
  );

  await packagePlugin(
    "auth",
    path.join(__dirname, "../../plugins/auth-plugin"),
    "libauth_plugin.dylib",
  );

  console.log("\n✨ 插件打包完成!\n");
}

main().catch(console.error);
