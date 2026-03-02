#!/usr/bin/env node

/**
 * 插件打包脚本
 *
 * 将插件打包为 .wtplugin.zip 文件
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const plugins = [
  {
    id: 'password-manager',
    dir: path.join(__dirname, '../../plugins/password-manager'),
    cargoDir: path.join(__dirname, '../../plugins/password-manager'),
  },
  {
    id: 'auth',
    dir: path.join(__dirname, '../../plugins/auth-plugin'),
    cargoDir: path.join(__dirname, '../../plugins/auth-plugin'),
  },
];

function buildPlugin(plugin) {
  console.log(`\n🔨 编译插件: ${plugin.id}`);

  try {
    // 编译 Rust 动态库
    console.log('  📦 编译 Rust 代码...');
    execSync('cargo build --release', {
      cwd: plugin.cargoDir,
      stdio: 'inherit',
    });
    console.log('  ✅ Rust 编译成功');
  } catch (error) {
    console.error(`  ❌ 编译失败: ${error.message}`);
    return false;
  }

  return true;
}

function packagePlugin(plugin) {
  console.log(`\n📦 打包插件: ${plugin.id}`);

  const outputDir = path.join(__dirname, '../dist-plugins');
  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
  }

  const outputFile = path.join(outputDir, `${plugin.id}.wtplugin.zip`);
  const archive = require('archiver')('zip', {
    zlib: { level: 9 }
  });

  const output = fs.createWriteStream(outputFile);
  archive.pipe(output);

  // 添加 manifest.json
  const manifestPath = path.join(plugin.dir, 'manifest.json');
  if (fs.existsSync(manifestPath)) {
    archive.file('manifest.json', fs.readFileSync(manifestPath));
    console.log('  ✓ 添加 manifest.json');
  }

  // 添加动态库
  const dylibName = process.platform === 'darwin'
    ? `lib${plugin.id.replace('-', '_')}.dylib`
    : process.platform === 'win32'
    ? `${plugin.id}.dll`
    : `lib${plugin.id.replace('-', '_')}.so`;

  const dylibPath = path.join(plugin.cargoDir, 'target/release', dylibName);
  if (fs.existsSync(dylibPath)) {
    archive.file(dylibName, fs.readFileSync(dylibPath));
    console.log(`  ✓ 添加 ${dylibName}`);
  } else {
    console.warn(`  ⚠ 动态库不存在: ${dylibPath}`);
  }

  // 添加 assets 目录
  const assetsDir = path.join(plugin.dir, 'assets');
  if (fs.existsSync(assetsDir)) {
    archive.directory(assetsDir, 'assets');
    console.log('  ✓ 添加 assets 目录');
  }

  return new Promise((resolve, reject) => {
    output.on('close', () => {
      console.log(`  ✅ 打包完成: ${outputFile}`);
      resolve();
    });
    archive.on('error', (err) => {
      console.error(`  ❌ 打包失败: ${err.message}`);
      reject(err);
    });
    archive.finalize();
  });
}

async function main() {
  console.log('🚀 开始插件打包流程...\n');

  // 安装依赖
  try {
    require('archiver');
  } catch {
    console.log('📦 安装依赖...');
    execSync('npm install archiver', {
      cwd: __dirname,
      stdio: 'inherit',
    });
  }

  // 构建并打包所有插件
  for (const plugin of plugins) {
    const buildSuccess = buildPlugin(plugin);
    if (!buildSuccess) {
      console.error(`\n❌ 插件 ${plugin.id} 构建失败,跳过打包`);
      continue;
    }

    try {
      await packagePlugin(plugin);
    } catch (error) {
      console.error(`\n❌ 插件 ${plugin.id} 打包失败: ${error.message}`);
    }
  }

  console.log('\n✨ 所有插件处理完成!\n');
}

main().catch(console.error);
