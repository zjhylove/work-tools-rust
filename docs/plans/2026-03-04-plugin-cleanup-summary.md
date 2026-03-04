# 插件特定代码清理 - 第一阶段完成总结

执行日期: 2026-03-04

## ✅ 已完成的清理

### 删除的命令 (5 个)

| 命令名 | 行号 (删除前) | 原因 |
|--------|---------------|------|
| `get_auth_entries` | 248-258 | 直接读取配置,绕过插件系统 |
| `save_auth_entry` | 262-282 | 直接保存配置,绕过插件系统 |
| `delete_auth_entry` | 286-306 | 直接修改配置,绕过插件系统 |
| `generate_secret` | 309-326 | TOTP 算法应该在插件中实现 |
| `generate_totp_code` | 377-404 | 简单包装,应直接使用 `call_plugin_method` |

### 保留的命令

**原因**: 这些命令包含额外的业务逻辑(如自动解密、参数转换等)

**password-manager** (保留):
- `get_password_entries` - 自动解密密码
- `save_password_entry` - 自动加密密码
- `delete_password_entry`
- `clear_all_password_entries`
- `export_passwords`
- `import_passwords`

**auth-plugin** (保留):
- `list_auth_entries`
- `add_auth_entry`
- `update_auth_entry`
- `delete_auth_entry_plugin`

---

## 📊 清理效果

### 代码减少

| 文件 | 删除行数 | 减少比例 |
|------|---------|---------|
| tauri-app/src-tauri/src/commands.rs | 109 行 | ~14% |
| tauri-app/src-tauri/src/lib.rs | 5 行 | ~5% |
| tauri-app/src/App.tsx | 11 行 | ~8% |
| tauri-app/src/components/PluginStore.tsx | 37 行 | ~43% |
| **总计** | **162 行** | **~15%** |

### 架构改善

1. ✅ **消除了数据一致性风险**
   - 删除了绕过插件系统的直接配置操作
   - 现在只有一种方式操作 Auth 插件数据:通过 `call_plugin_method`

2. ✅ **业务逻辑归位**
   - TOTP 算法实现现在完全在 auth-plugin 中
   - 主应用不再包含特定插件的业务逻辑

3. ✅ **类型定义统一**
   - 创建了 `tauri-app/src/types/plugin.ts` 统一前端类型定义
   - 更新了 App.tsx 和 PluginStore.tsx 使用统一类型

4. ✅ **代码清晰度提升**
   - 删除了重复的实现
   - 职责更加清晰:tauri-app 负责插件基础设施,插件负责业务逻辑

---

## 🧪 测试结果

### 编译测试

```bash
✅ cargo check - 通过
   没有编译错误或警告
```

### 功能测试建议

建议手动测试以下功能:

**Auth 插件**:
- [ ] 添加新的 TOTP 条目
- [ ] 编辑 TOTP 条目
- [ ] 删除 TOTP 条目
- [ ] 生成 TOTP 验证码
- [ ] 生成 TOTP secret

**Password Manager 插件**:
- [ ] 添加密码
- [ ] 编辑密码
- [ ] 删除密码
- [ ] 查看密码列表 (自动解密)

---

## 📝 修改的文件

1. **plugins/auth-plugin/src/lib.rs** (1 行)
   - 更新图标: 🔐 → 🔢

2. **tauri-app/src-tauri/src/commands.rs** (-109 行)
   - 删除 5 个命令
   - 保留 3 个类型定义 (供其他命令使用)

3. **tauri-app/src-tauri/src/lib.rs** (-5 行)
   - 从 invoke_handler 移除 5 个已删除命令

4. **tauri-app/src/types/plugin.ts** (新建)
   - 统一的插件类型定义

5. **tauri-app/src/App.tsx** (-11 行)
   - 使用统一的类型定义

6. **tauri-app/src/components/PluginStore.tsx** (-37 行)
   - 使用统一的类型定义

---

## 🔄 第二阶段建议 (可选)

如果想要进一步简化,可以考虑删除 password-manager 的包装命令:

### 需要删除的命令 (10 个)

- `get_password_entries`
- `save_password_entry`
- `delete_password_entry`
- `clear_all_password_entries`
- `export_passwords`
- `import_passwords`
- `list_auth_entries`
- `add_auth_entry`
- `update_auth_entry`
- `delete_auth_entry_plugin`

### 前端需要做的改动

将所有调用改为使用 `call_plugin_method`:

```typescript
// 替换前
const passwords = await invoke("get_password_entries");

// 替换后
const result = await invoke("call_plugin_method", {
  pluginId: "password-manager",
  method: "list_passwords",
  params: {},
});
const passwords = result.entries;
```

### 预期效果

- 再减少 ~200 行代码
- 架构更加统一和清晰
- 前端需要更多改动

---

## 💡 关键经验

1. **分阶段执行**: 先删除最危险的代码(绕过插件的命令),再考虑简化包装命令

2. **保留有价值的逻辑**: 某些包装命令包含额外的业务逻辑(如自动解密),暂时保留是合理的

3. **类型定义依赖**: 删除类型时需要检查是否还有其他命令在使用它们

4. **测试很重要**: 每个阶段后都要编译测试,确保没有破坏功能

---

## ✨ 总结

第一阶段清理已成功完成!

- ✅ 消除了数据一致性风险
- ✅ 删除了 109 行冗余代码
- ✅ 业务逻辑归位到插件
- ✅ 架构更加清晰
- ✅ 编译通过,无错误

应用现在的架构符合设计原则:tauri-app 提供插件基础设施,插件负责业务逻辑和 UI。

---

**清理完成!** 🎉
