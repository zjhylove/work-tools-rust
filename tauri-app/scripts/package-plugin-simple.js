#!/usr/bin/env node

/**
 * 简化版插件打包脚本
 *
 * 仅打包 manifest.json 和 assets,不包含编译后的动态库
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

async function packagePlugin(pluginId, pluginDir) {
  console.log(`\n📦 打包插件: ${pluginId}`);

  const outputDir = path.join(__dirname, '../dist-plugins');
  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
  }

  const outputFile = path.join(outputDir, `${pluginId}.wtplugin.zip`);

  // 使用系统 zip 命令
  try {
    const files = [
      'manifest.json',
      'assets/styles.css',
      'assets/index.html'
    ].map(f => path.join(pluginDir, f)).filter(f => fs.existsSync(f));

    if (files.length === 0) {
      console.log('  ⚠ 没有找到文件');
      return;
    }

    const zipArgs = ['-r', outputFile, ...files.map(f => f.replace(pluginDir + '/', ''))];
    execSync(`zip ${zipArgs.join(' ')}`, {
      cwd: pluginDir,
      stdio: 'inherit'
    });

    console.log(`  ✅ 打包完成: ${outputFile}`);
  } catch (error) {
    console.error(`  ❌ 打包失败: ${error.message}`);
  }
}

async function main() {
  console.log('🚀 开始插件打包...\n');

  // 先构建前端资源
  console.log('📝 构建前端资源...');
  execSync('npm run build-plugins', {
    cwd: __dirname,
    stdio: 'inherit'
  });

  // 打包插件
  await packagePlugin('password-manager', path.join(__dirname, '../../plugins/password-manager'));
  await packagePlugin('auth', path.join(__dirname, '../../plugins/auth-plugin'));

  console.log('\n✨ 插件打包完成!\n');
}

main().catch(console.error);
