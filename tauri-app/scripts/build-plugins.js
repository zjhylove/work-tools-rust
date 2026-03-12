#!/usr/bin/env node

/**
 * 插件打包脚本
 *
 * 将前端组件的 CSS 复制到插件 assets 目录
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const plugins = [
  {
    id: 'password-manager',
    cssSource: path.join(__dirname, '../src/components/PasswordManager.css'),
    assetsDir: path.join(__dirname, '../../plugins/password-manager/assets'),
  },
  {
    id: 'auth',
    cssSource: path.join(__dirname, '../src/components/AuthPlugin.css'),
    assetsDir: path.join(__dirname, '../../plugins/auth-plugin/assets'),
  },
];

function copyFile(source, dest) {
  const destDir = path.dirname(dest);
  if (!fs.existsSync(destDir)) {
    fs.mkdirSync(destDir, { recursive: true });
  }
  fs.copyFileSync(source, dest);
  console.log(`✓ 复制: ${path.basename(dest)}`);
}

function buildPluginAssets() {
  console.log('🔨 开始构建插件前端资源...\n');

  plugins.forEach((plugin) => {
    console.log(`\n📦 处理插件: ${plugin.id}`);

    // 确保 assets 目录存在
    if (!fs.existsSync(plugin.assetsDir)) {
      fs.mkdirSync(plugin.assetsDir, { recursive: true });
      console.log(`  ✓ 创建 assets 目录`);
    }

    // 复制 CSS
    const cssDest = path.join(plugin.assetsDir, 'styles.css');
    if (fs.existsSync(plugin.cssSource)) {
      copyFile(plugin.cssSource, cssDest);
    } else {
      console.warn(`  ⚠ CSS 文件不存在: ${plugin.cssSource}`);
    }

    console.log(`  ✅ 插件 ${plugin.id} 处理完成`);
  });

  console.log('\n✨ 所有插件前端资源构建完成!\n');
}

buildPluginAssets();
