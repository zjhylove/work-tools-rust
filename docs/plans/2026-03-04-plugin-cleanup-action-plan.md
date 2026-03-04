# 插件特定代码清理执行计划

## 目标

从 tauri-app 中移除所有插件特定的业务逻辑,只保留通用的插件基础设施。

## 当前状况

### ✅ 正确的实现 (使用通用命令)

这些命令正确地使用了 `call_plugin_method`:

```rust
// 密码管理器 - 通过插件调用 ✅
pub async fn get_password_entries(...) {
    manager.call_plugin_method("password-manager", "list_passwords", ...).await
}

pub async fn save_password_entry(...) {
    manager.call_plugin_method("password-manager", method, params).await
}

pub async fn delete_password_entry(...) {
    manager.call_plugin_method("password-manager", "delete_password", ...).await
}

// Auth 插件 - 通过插件调用 ✅
pub async fn list_auth_entries(...) {
    manager.call_plugin_method("auth", "list_entries", ...).await
}

pub async fn add_auth_entry(...) {
    manager.call_plugin_method("auth", "add_entry", ...).await
}

pub async fn update_auth_entry(...) {
    manager.call_plugin_method("auth", "update_entry", ...).await
}

pub async fn delete_auth_entry_plugin(...) {
    manager.call_plugin_method("auth", "delete_entry", ...).await
}
```

### ❌ 错误的实现 (绕过插件,直接操作数据)

这些命令**绕过了插件系统**,直接读取/修改配置:

```rust
// Auth 插件 - 直接操作配置 ❌
pub async fn get_auth_entries() -> Result<Vec<AuthEntry>, String> {
    let config = load_plugin_config("auth")?;  // 绕过插件!
    let entries: Vec<AuthEntry> = config.get("entries")...
    Ok(entries)
}

pub async fn save_auth_entry(entry: AuthEntry) -> Result<(), String> {
    let mut config = load_plugin_config("auth")?;  // 绕过插件!
    // ... 直接修改配置
    save_plugin_config("auth", &config)?;
}
```

### ⚠️ 业务逻辑泄露

这些命令在主应用中实现了插件的业务逻辑:

```rust
// ❌ TOTP 算法应该在 auth-plugin 中
pub async fn generate_secret() -> Result<String, String> {
    use ::otp::totp::TOTP;  // TOTP 算法实现
    let secret = TOTP::default()...
}

// ❌ TOTP 代码生成应该在 auth-plugin 中
pub async fn generate_totp_code(secret: String, ...) -> Result<String, String> {
    use ::otp::totp::TOTP;
    // ... TOTP 算法
}
```

## 清理策略

### 阶段 1: 删除绕过插件的命令 (优先级: 高)

#### 列表

| 文件 | 行号 | 命令名 | 原因 |
|------|------|--------|------|
| commands.rs | 248-258 | `get_auth_entries` | 直接读取配置,绕过插件 |
| commands.rs | 262-282 | `save_auth_entry` | 直接保存配置,绕过插件 |
| commands.rs | 286-306 | `delete_auth_entry` | 直接修改配置,绕过插件 |

#### 替代方案

这些功能已经有正确的实现:
- `get_auth_entries` → 使用 `list_auth_entries` (行 387-403)
- `save_auth_entry` → 使用 `add_auth_entry` 或 `update_auth_entry` (行 406-439)
- `delete_auth_entry` → 使用 `delete_auth_entry_plugin` (行 442-454)

---

### 阶段 2: 删除业务逻辑泄露的命令 (优先级: 高)

#### 列表

| 文件 | 行号 | 命令名 | 原因 |
|------|------|--------|------|
| commands.rs | 309-326 | `generate_secret` | TOTP 算法应该在 auth-plugin 中 |
| commands.rs | 457-487 | `generate_totp_code` | TOTP 算法应该在 auth-plugin 中 |

#### 需要在 auth-plugin 中添加

确保 auth-plugin 实现了以下方法:
- `generate_secret` - 生成 TOTP secret
- `generate_code` - 生成 TOTP 验证码

---

### 阶段 3: 简化包装命令 (优先级: 中)

#### 列表

这些命令只是简单地包装了 `call_plugin_method`,可以考虑删除:

| 文件 | 行号 | 命令名 | 建议 |
|------|------|--------|------|
| commands.rs | 119-172 | `get_password_entries` | 前端直接使用 `call_plugin_method` |
| commands.rs | 174-218 | `save_password_entry` | 前端直接使用 `call_plugin_method` |
| commands.rs | 220-231 | `delete_password_entry` | 前端直接使用 `call_plugin_method` |
| commands.rs | 234-244 | `clear_all_password_entries` | 前端直接使用 `call_plugin_method` |
| commands.rs | 354-369 | `export_passwords` | 前端直接使用 `call_plugin_method` |
| commands.rs | 371-385 | `import_passwords` | 前端直接使用 `call_plugin_method` |
| commands.rs | 387-403 | `list_auth_entries` | 前端直接使用 `call_plugin_method` |
| commands.rs | 406-421 | `add_auth_entry` | 前端直接使用 `call_plugin_method` |
| commands.rs | 424-439 | `update_auth_entry` | 前端直接使用 `call_plugin_method` |
| commands.rs | 442-454 | `delete_auth_entry_plugin` | 前端直接使用 `call_plugin_method` |

#### 前端使用方式

**当前 (包装命令)**:
```typescript
const passwords = await invoke("get_password_entries");
```

**改进后 (通用命令)**:
```typescript
const result = await invoke("call_plugin_method", {
  pluginId: "password-manager",
  method: "list_passwords",
  params: {},
});
const passwords = result.entries;
```

---

### 阶段 4: 删除插件特定类型定义 (优先级: 低)

#### 列表

| 文件 | 行号 | 类型名 | 原因 |
|------|------|--------|------|
| commands.rs | 19-27 | `PasswordEntry` | 应该在 password-manager 插件中 |
| commands.rs | 29-37 | `DecryptedPasswordEntry` | 应该在 password-manager 插件中 |
| commands.rs | 39-48 | `AuthEntry` | 应该在 auth-plugin 中 |

#### 影响

这些类型只被已删除的命令使用,删除它们不会影响其他代码。

---

### 阶段 5: 独立加密服务 (优先级: 低)

#### 列表

| 文件 | 行号 | 命令名 | 操作 |
|------|------|--------|------|
| commands.rs | 328-339 | `encrypt_password` | 移到 crypto_service.rs |
| commands.rs | 341-352 | `decrypt_password` | 移到 crypto_service.rs |

这些是通用服务,可以保留但应该独立到自己的模块。

---

## 执行步骤

### Step 1: 检查插件是否已实现所有必要方法

#### password-manager

检查 password-manager 插件是否实现了:
- [ ] `list_passwords`
- [ ] `add_password`
- [ ] `update_password`
- [ ] `delete_password`
- [ ] `clear_all_passwords`
- [ ] `export_passwords`
- [ ] `import_passwords`

#### auth-plugin

检查 auth-plugin 是否实现了:
- [ ] `list_entries`
- [ ] `add_entry`
- [ ] `update_entry`
- [ ] `delete_entry`
- [ ] `generate_secret`
- [ ] `generate_code` (新增)

### Step 2: 更新前端代码

#### password-manager/frontend

搜索并替换所有调用:
```typescript
// 替换前
invoke("get_password_entries")
invoke("save_password_entry", { entry })
invoke("delete_password_entry", { id })

// 替换后
invoke("call_plugin_method", {
  pluginId: "password-manager",
  method: "list_passwords",
  params: {}
})
invoke("call_plugin_method", {
  pluginId: "password-manager",
  method: "add_password",
  params: { entry }
})
invoke("call_plugin_method", {
  pluginId: "password-manager",
  method: "delete_password",
  params: { id }
})
```

#### auth-plugin/frontend

搜索并替换所有调用:
```typescript
// 替换前
invoke("get_auth_entries")
invoke("save_auth_entry", { entry })
invoke("generate_totp_code", { secret })

// 替换后
invoke("call_plugin_method", {
  pluginId: "auth",
  method: "list_entries",
  params: {}
})
invoke("call_plugin_method", {
  pluginId: "auth",
  method: "add_entry",
  params: { entry }
})
invoke("call_plugin_method", {
  pluginId: "auth",
  method: "generate_code",
  params: { secret }
})
```

### Step 3: 更新 auth-plugin 添加缺失方法

如果 auth-plugin 缺少以下方法,需要添加:

```rust
// plugins/auth-plugin/src/lib.rs

impl Plugin for AuthPlugin {
    fn handle_call(&mut self, method: &str, params: Value) -> Result<Value, Box<dyn Error>> {
        match method {
            // ... 现有方法 ...

            "generate_secret" => {
                use ::otp::totp::TOTP;
                let secret = TOTP::default()
                    .generate_secret()?
                    .to_string();
                Ok(serde_json::json!({ "secret": secret }))
            }

            "generate_code" => {
                let secret = params.get("secret")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 secret 参数"))?;

                let timestamp = params.get("timestamp")
                    .and_then(|v| v.as_u64())
                    .unwrap_or_else(|| {
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                    });

                let totp = TOTP::default();
                let code = totp.generate_code(secret, timestamp)?;
                Ok(serde_json::json!({ "code": code, "valid_until": timestamp + 30 }))
            }

            _ => Err("unknown method".into()),
        }
    }
}
```

### Step 4: 删除后端命令

#### 删除阶段 1 的命令 (绕过插件的命令)

从 `commands.rs` 删除:
- 行 245-306: `get_auth_entries`, `save_auth_entry`, `delete_auth_entry`

#### 删除阶段 2 的命令 (业务逻辑泄露)

从 `commands.rs` 删除:
- 行 309-326: `generate_secret`
- 行 457-487: `generate_totp_code`

#### 删除阶段 3 的命令 (包装命令) - 可选

如果你希望简化,可以删除这些包装命令:
- 行 119-233: 所有 password-manager 包装命令
- 行 387-454: 所有 auth-plugin 包装命令

#### 删除阶段 4 的类型定义

从 `commands.rs` 开头删除:
- `PasswordEntry`
- `DecryptedPasswordEntry`
- `AuthEntry`

### Step 5: 更新 lib.rs

从 `tauri-app/src-tauri/src/lib.rs` 中移除已删除命令的注册:

```rust
// 删除这些行
.invoke_handler(tauri::generate_handler![
    // ...
    get_password_entries,        // 删除
    save_password_entry,         // 删除
    delete_password_entry,       // 删除
    clear_all_password_entries,  // 删除
    get_auth_entries,            // 删除
    save_auth_entry,             // 删除
    delete_auth_entry,           // 删除
    generate_secret,             // 删除
    generate_totp_code,          // 删除
    // ...
])
```

### Step 6: 测试

#### 单元测试

```bash
# 测试 password-manager 插件
cargo test -p password-manager

# 测试 auth-plugin 插件
cargo test -p auth-plugin
```

#### 集成测试

```bash
# 启动应用
cd tauri-app
npm run tauri dev

# 手动测试:
# 1. 密码管理器 - 添加/编辑/删除密码
# 2. 双因素认证 - 添加/编辑/删除 TOTP
# 3. TOTP 代码生成
```

---

## 预期效果

### 代码减少

| 项目 | 删除前 | 删除后 | 减少 |
|------|--------|--------|------|
| commands.rs | ~770 行 | ~400 行 | ~370 行 (48%) |
| 插件特定命令 | 15 个 | 0 个 | -15 个 |
| 插件特定类型 | 3 个 | 0 个 | -3 个 |

### 架构改善

1. ✅ **清晰的职责分离**
   - tauri-app: 插件基础设施
   - 插件: 业务逻辑

2. ✅ **消除数据一致性风险**
   - 只有一种方式操作插件数据
   - 删除了双重实现

3. ✅ **插件自治**
   - 插件完全控制自己的数据和业务逻辑
   - 插件可以独立演进

---

## 风险和缓解

### 风险 1: 前端仍在使用已删除的命令

**缓解**:
- 在删除前搜索所有使用点
- 更新所有前端代码
- 运行测试确保功能正常

### 风险 2: 插件缺少某些方法

**缓解**:
- 先检查插件实现了哪些方法
- 添加缺失的方法到插件
- 测试插件功能

### 风险 3: 加密/解密逻辑移动

**缓解**:
- `encrypt_password` 和 `decrypt_password` 保留在主应用
- 作为通用服务供插件使用

---

## 下一步

1. **检查插件实现** - 验证插件已实现所有必要方法
2. **搜索使用点** - 找出所有使用待删除命令的地方
3. **开始清理** - 按阶段逐步删除不需要的代码

准备好开始了吗?我可以帮你执行任何步骤。
