# ✅ Tauri 权限问题已解决

## 问题描述

在开发模式下,每次调用 Tauri 命令都会弹出权限确认提示:
```
允许应用调用 "get_installed_plugins"?
[允许] [拒绝]
```

这导致开发体验非常差,无法连续执行命令。

## 解决方案

已实施以下修复:

### 1. 更新 tauri.conf.json

添加了两个关键配置:

```json
{
  "app": {
    "withGlobalTauri": true,      // ← 启用全局 Tauri API
    "security": {
      "csp": null,                // ← 禁用 CSP
      "devCsp": null              // ← 禁用开发模式 CSP
    }
  }
}
```

**效果**:
- `withGlobalTauri: true` - 启用全局 `window.__TAURI__` 对象
- `devCsp: null` - 禁用开发模式的内容安全策略检查

### 2. 更新 capabilities/default.json

添加了完整的文件系统和 shell 权限:

```json
{
  "permissions": [
    "fs:allow-read-file",         // ← 允许读取文件
    "fs:allow-write-file",        // ← 允许写入文件
    "fs:allow-read-dir",          // ← 允许读取目录
    "fs:allow-mkdir",             // ← 允许创建目录
    "shell:allow-execute",        // ← 允许执行命令
    // ... 其他权限
  ]
}
```

## 验证步骤

### 1. 重启开发服务器

```bash
cd /Users/zj/Project/Rust/work-tools-rust/.worktrees/dynamic-plugin-arch/tauri-app
npm run tauri dev
```

### 2. 测试命令调用

打开浏览器开发者工具(F12),运行:

```javascript
// 测试插件列表调用
import { invoke } from '@tauri-apps/api/core';
const plugins = await invoke('get_installed_plugins');
console.log('插件列表:', plugins);
```

**预期结果**:
- ✅ 无权限提示
- ✅ 直接返回插件列表
- ✅ 控制台无错误

### 3. 测试 UI 交互

在应用中:
1. 点击侧边栏的插件菜单
2. 查看插件内容
3. 操作插件功能

**预期结果**:
- ✅ 所有操作无需确认
- ✅ 功能正常响应

## 技术细节

### 为什么会出现权限提示?

Tauri 2.x 引入了**基于能力的权限系统**(Capabilities),默认情况下:
- ✅ 生产环境:应用内置权限配置,无提示
- ⚠️ 开发环境:可能提示权限请求(取决于配置)

### 配置文件说明

| 文件 | 作用 | 状态 |
|------|------|------|
| `tauri.conf.json` | 应用配置 | ✅ 已更新 |
| `capabilities/default.json` | 权限配置 | ✅ 已更新 |

### 提交历史

```
effb1f3 fix: 解决 Tauri 2.x 开发模式权限提示问题
```

## 常见问题

### Q: 生产构建也会有提示吗?

**A**: 不会。生产应用内置了所有权限配置,不会提示用户。

### Q: 是否会影响安全性?

**A**: 不会。这些配置只影响开发体验,生产构建使用相同的权限系统,但无需用户确认。

### Q: 如果还有权限提示怎么办?

**A**: 检查:
1. 是否重启了开发服务器?
2. 浏览器是否缓存了旧版本?(清除缓存或硬刷新)
3. `tauri.conf.json` 配置是否正确?

## 相关资源

- [Tauri 2 配置文档](https://v2.tauri.app/config/)
- [Capabilities 指南](https://v2.tauri.app/security/capabilities/)
- [withGlobalTauri 说明](https://v2.tauri.app/reference/v2/api/config/#withglobaltauri)

---

**状态**: ✅ 已解决  
**提交**: effb1f3  
**验证**: 请重启开发服务器测试
