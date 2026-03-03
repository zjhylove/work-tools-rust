# 修复密码管理器保存/更新密码失败的问题

## 问题描述
在密码管理器中保存或更新密码时失败,控制台提示:
```
"插件方法调用失败: 未知方法: save_password"
```

## 根本原因
前端调用的方法名 (`save_password`) 与插件实现的方法名不匹配。

插件中实现的方法:
- `add_password` - 添加新密码
- `update_password` - 更新现有密码
- `delete_password` - 删除密码
- `list_passwords` - 列出所有密码

但前端统一调用了不存在的 `save_password` 方法。

## 解决方案

### 修改前端代码
在 `plugins/password-manager/frontend/src/App.tsx` 的 `handleAction` 函数中:

**修改前**:
```typescript
await window.pluginAPI?.call("password-manager", "save_password", {
  entry,
});
```

**修改后**:
```typescript
// 根据是否是编辑模式调用不同的方法
if (isEdit && selectedEntry) {
  // 更新现有密码
  await window.pluginAPI?.call("password-manager", "update_password", {
    id: entry.id,
    service: entry.service,
    username: entry.username,
    password: entry.password,
    url: entry.url,
  });
} else {
  // 添加新密码
  await window.pluginAPI?.call("password-manager", "add_password", {
    service: entry.service,
    username: entry.username,
    password: entry.password,
    url: entry.url,
  });
}
```

### 重新构建插件

1. **构建前端资源**:
```bash
cd plugins/password-manager/frontend
npm run build
```

2. **打包插件**:
```bash
cd ..
zip -r password-manager.wtplugin.zip manifest.json libpassword_manager.dylib assets/
```

3. **重新导入插件**:
   - 启动 Work Tools 应用
   - 点击插件商店按钮 (🧩)
   - 先卸载旧版本的密码管理器插件
   - 点击"导入插件"
   - 选择新的 `password-manager.wtplugin.zip` 文件

## API 参数说明

### add_password
添加新密码条目。

**参数**:
- `service` (string, 必填): 服务名称
- `username` (string, 必填): 用户名
- `password` (string, 必填): 密码
- `url` (string, 可选): 相关 URL

**返回**: 新创建的密码条目对象

### update_password
更新现有密码条目。

**参数**:
- `id` (string, 必填): 密码条目 ID
- `service` (string, 必填): 服务名称
- `username` (string, 必填): 用户名
- `password` (string, 必填): 密码
- `url` (string, 可选): 相关 URL

**返回**: 更新后的密码条目对象

### delete_password
删除密码条目。

**参数**:
- `id` (string, 必填): 密码条目 ID

**返回**: `{ success: true }`

### list_passwords
列出所有密码条目。

**参数**: 无

**返回**: 密码条目数组

## 测试步骤

### 测试添加密码
1. 打开密码管理器
2. 点击"添加密码"按钮
3. 填写表单:
   - 服务名称: "测试服务"
   - 用户名: "test@example.com"
   - 密码: "password123"
   - URL: "https://example.com"
4. 点击"保存"
5. 验证密码出现在列表中

### 测试更新密码
1. 在密码列表中点击刚才添加的密码
2. 修改任意字段(如密码改为 "newpassword456")
3. 点击"保存"
4. 验证密码已更新

### 测试删除密码
1. 在密码列表中点击密码条目
2. 点击"删除"按钮
3. 确认删除
4. 验证密码已从列表中移除

## 相关文件
- `/Users/zj/Project/Rust/work-tools-rust/plugins/password-manager/frontend/src/App.tsx`
- `/Users/zj/Project/Rust/work-tools-rust/plugins/password-manager/src/lib.rs`

## 插件包位置
- 源码: `/Users/zj/Project/Rust/work-tools-rust/plugins/password-manager/`
- 插件包: `plugins/password-manager/password-manager.wtplugin.zip` (362 KB)
- 安装位置: `~/.worktools/plugins/password-manager/`

## 后续优化建议
1. 考虑在插件中添加 `save_password` 方法作为 `add_password` 和 `update_password` 的统一封装
2. 添加密码强度检测
3. 添加密码复制功能
4. 添加密码过期提醒
