# Tauri 权限提示问题解决方案

## 问题

在开发模式下,每次调用 Tauri 命令都会弹出权限确认提示。

## 原因

Tauri 2.x 采用了严格的权限系统(capabilities),默认情况下会提示用户授权。

## 解决方案(选择其一)

### 方案 1: 更新 tauri.conf.json(推荐,最简单)

在 `tauri.conf.json` 中添加 `security` 配置,禁用开发模式的权限提示:

```json
{
  "app": {
    "windows": [...],
    "security": {
      "csp": null,
      "devCsp": null
    }
  }
}
```

### 方案 2: 使用简化模式 capabilities(推荐,用于开发)

在 `tauri.conf.json` 中指定 capabilities 目录并使用宽松配置:

```json
{
  "app": {
    "security": {
      "csp": null
    }
  },
  "bundles": {
    "active": true,
    "targets": "all",
    "icon": [...]
  }
}
```

然后在 `capabilities/default.json` 中确保包含所有需要的权限。

### 方案 3: 创建开发模式 capabilities(最灵活)

创建 `capabilities/dev.json`:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "dev",
  "description": "Development capability - allows all commands",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:default",
    "core:path:default",
    "fs:default",
    "shell:default",
    "dialog:default",
    "opener:default"
  ]
}
```

然后在 `tauri.conf.json` 中:

```json
{
  "app": {
    "windows": [...],
    "withGlobalTauri": true,
    "security": {
      "csp": null
    }
  }
}
```

## 已实施的更改

✅ 更新了 `capabilities/default.json`,添加了所有基础权限
✅ 包含 `fs:allow-read-file`, `fs:allow-write-file` 等

## 验证步骤

1. 重新启动开发服务器:
   ```bash
   cd tauri-app
   npm run tauri dev
   ```

2. 测试命令调用:
   - 点击侧边栏插件菜单
   - 查看是否还有权限提示

3. 如果仍有提示,检查:
   - 浏览器控制台是否有错误
   - Tauri DevTools 中是否有安全警告

## 注意事项

- 生产构建时,应该使用更严格的权限配置
- 开发模式可以使用宽松权限
- 最终用户不应看到权限提示(已内置在应用中)

## 相关文档

- [Tauri 2 Capabilities](https://v2.tauri.app/security/capabilities/)
- [Tauri 2 Permission System](https://v2.tauri.app/security/)
