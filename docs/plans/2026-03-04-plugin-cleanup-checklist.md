# 插件特定代码清理 - 执行清单

## ✅ 前置条件检查

### 插件方法实现检查

| 插件 | 方法 | 状态 |
|------|------|------|
| password-manager | list_passwords | ✅ 已实现 |
| password-manager | add_password | ✅ 已实现 |
| password-manager | update_password | ✅ 已实现 |
| password-manager | delete_password | ✅ 已实现 |
| password-manager | clear_all_passwords | ✅ 已实现 |
| password-manager | export_passwords | ✅ 已实现 |
| password-manager | import_passwords | ✅ 已实现 |
| auth-plugin | list_entries | ✅ 已实现 |
| auth-plugin | add_entry | ✅ 已实现 |
| auth-plugin | update_entry | ✅ 已实现 |
| auth-plugin | delete_entry | ✅ 已实现 |
| auth-plugin | generate_secret | ✅ 已实现 |
| auth-plugin | generate_code | ⚠️ 需要检查 |

### 前端使用检查

| 检查项 | 结果 |
|--------|------|
| password-manager/frontend 使用包装命令 | ✅ 未使用 |
| auth-plugin/frontend 使用包装命令 | ✅ 未使用 |
| tauri-app/src 使用包装命令 | ⚠️ 需要检查 |

---

## 📋 清理任务清单

### 任务 1: 删除绕过插件的命令 (高优先级)

**文件**: `tauri-app/src-tauri/src/commands.rs`

- [ ] 删除 `get_auth_entries` (行 248-258)
- [ ] 删除 `save_auth_entry` (行 262-282)
- [ ] 删除 `delete_auth_entry` (行 286-306)

**原因**: 这些命令直接读取/修改配置,绕过了插件系统,与正确的实现 (通过 `call_plugin_method`) 重复。

---

### 任务 2: 删除业务逻辑泄露的命令 (高优先级)

**文件**: `tauri-app/src-tauri/src/commands.rs`

- [ ] 删除 `generate_secret` (行 309-326)
- [ ] 删除 `generate_totp_code` (行 457-487)

**原因**: TOTP 算法实现应该在 auth-plugin 插件中,不应该在主应用中。

**注意**: 需要先检查 auth-plugin 是否已经实现了 `generate_code` 方法。

---

### 任务 3: 简化包装命令 (中优先级)

**文件**: `tauri-app/src-tauri/src/commands.rs`

**选项 A: 删除所有包装命令** (推荐)

删除以下命令,让前端直接使用 `call_plugin_method`:

**password-manager**:
- [ ] 删除 `get_password_entries` (行 119-172)
- [ ] 删除 `save_password_entry` (行 174-218)
- [ ] 删除 `delete_password_entry` (行 220-231)
- [ ] 删除 `clear_all_password_entries` (行 234-244)
- [ ] 删除 `export_passwords` (行 354-369)
- [ ] 删除 `import_passwords` (行 371-385)

**auth-plugin**:
- [ ] 删除 `list_auth_entries` (行 387-403)
- [ ] 删除 `add_auth_entry` (行 406-421)
- [ ] 删除 `update_auth_entry` (行 424-439)
- [ ] 删除 `delete_auth_entry_plugin` (行 442-454)

**选项 B: 保留包装命令** (不推荐)

如果担心破坏现有功能,可以暂时保留这些命令。

---

### 任务 4: 删除插件特定类型定义 (低优先级)

**文件**: `tauri-app/src-tauri/src/commands.rs`

- [ ] 删除 `PasswordEntry` 结构体 (行 19-27)
- [ ] 删除 `DecryptedPasswordEntry` 结构体 (行 29-37)
- [ ] 删除 `AuthEntry` 结构体 (行 39-48)

**注意**: 这些类型只被已删除的命令使用,不影响其他代码。

---

### 任务 5: 更新 lib.rs

**文件**: `tauri-app/src-tauri/src/lib.rs`

- [ ] 从 `invoke_handler` 中移除已删除的命令

---

### 任务 6: 测试

- [ ] 运行 `cargo test -p password-manager`
- [ ] 运行 `cargo test -p auth-plugin`
- [ ] 运行 `cd tauri-app && npm run tauri dev`
- [ ] 手动测试密码管理器功能
- [ ] 手动测试双因素认证功能

---

## 🎯 推荐的执行顺序

### 第一阶段 (立即执行)

1. **任务 1**: 删除绕过插件的命令 (3 个命令)
2. **任务 2**: 删除业务逻辑泄露的命令 (2 个命令)
3. **任务 6**: 测试确保功能正常

**预期效果**: 消除数据一致性风险,减少 ~80 行代码

### 第二阶段 (后续执行)

4. **任务 3**: 删除所有包装命令 (10 个命令)
5. **任务 4**: 删除插件特定类型定义 (3 个类型)
6. **任务 5**: 更新 lib.rs
7. **任务 6**: 全面测试

**预期效果**: 架构清晰,减少 ~290 行代码,总计减少 ~370 行 (48%)

---

## ⚠️ 注意事项

1. **备份**: 在删除代码前,建议先创建 git commit 保存当前状态

2. **逐步删除**: 不要一次性删除所有代码,分阶段执行并测试

3. **搜索使用点**: 在删除每个命令前,搜索是否有其他代码使用它:
   ```bash
   grep -r "get_auth_entries" tauri-app/src
   grep -r "save_auth_entry" tauri-app/src
   ```

4. **auth-plugin generate_code**: 需要检查 auth-plugin 是否实现了 `generate_code` 方法,如果没实现需要添加

---

## 📊 预期效果总结

| 指标 | 第一阶段 | 第二阶段 | 总计 |
|------|---------|---------|------|
| 删除命令数 | 5 个 | 10 个 | 15 个 |
| 删除类型数 | 0 个 | 3 个 | 3 个 |
| 代码行数减少 | ~80 行 | ~290 行 | ~370 行 (48%) |
| 数据一致性风险 | 消除 | - | 消除 |
| 架构清晰度 | 提升 | 显著提升 | 显著提升 |

---

准备好开始清理了吗?
