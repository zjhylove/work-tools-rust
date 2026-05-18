# shared/types -- 共享数据类型

## 概述

`shared/types` 是主程序和插件之间共享的数据结构定义。放在独立 crate 中是为了避免循环依赖：主程序依赖 types，插件依赖 plugin-api，两者都可以同时依赖 types。

依赖：`serde`（Serialize / Deserialize）

## PluginInfo 结构体

插件向外部暴露的元信息集合。主程序通过此结构体获取插件的基本信息，用于侧边栏展示、插件注册表管理等场景。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub icon: String,
}
```

### 字段说明

| 字段 | 类型 | 说明 | 示例 |
|------|------|------|------|
| `id` | `String` | 插件唯一标识符，用于系统中标识插件，建议使用 kebab-case | `"password-manager"` |
| `name` | `String` | 插件显示名称，在前端 UI 中展示 | `"密码管理器"` |
| `version` | `String` | 版本号，遵循 SemVer | `"1.0.0"` |
| `description` | `String` | 功能描述 | `"安全存储和管理你的密码"` |
| `icon` | `String` | 图标，使用 emoji 字符 | `"🔐"` |

### derive 说明

- `Debug` -- 支持 `{:?}` 格式化输出，用于调试日志
- `Clone` -- 支持深拷贝，`PluginInfo` 可安全复制传递
- `Serialize` -- 自动序列化为 JSON（由 serde 提供）
- `Deserialize` -- 自动从 JSON 反序列化（由 serde 提供）

### 使用示例

#### 在插件中构造 PluginInfo

插件实现 `Plugin` trait 时，通常需要返回 `PluginInfo`：

```rust
use worktools_shared_types::PluginInfo;

fn plugin_info() -> PluginInfo {
    PluginInfo {
        id: "password-manager".to_string(),
        name: "密码管理器".to_string(),
        version: "1.0.0".to_string(),
        description: "安全存储和管理你的密码".to_string(),
        icon: "🔐".to_string(),
    }
}
```

#### 序列化为 JSON

```rust
use worktools_shared_types::PluginInfo;

let info = PluginInfo {
    id: "json-tools".to_string(),
    name: "JSON 工具".to_string(),
    version: "1.0.0".to_string(),
    description: "JSON 格式化、压缩、验证".to_string(),
    icon: "📋".to_string(),
};

// 序列化为 JSON 字符串
let json = serde_json::to_string(&info).unwrap();
// {"id":"json-tools","name":"JSON 工具","version":"1.0.0","description":"JSON 格式化、压缩、验证","icon":"📋"}

// 格式化输出
let pretty = serde_json::to_string_pretty(&info).unwrap();
```

#### 从 JSON 反序列化

```rust
use worktools_shared_types::PluginInfo;

let json = r#"{"id":"redis-client","name":"Redis 客户端","version":"1.0.0","description":"Redis 数据浏览与操作","icon":"🗄️"}"#;
let info: PluginInfo = serde_json::from_str(json).unwrap();

assert_eq!(info.id, "redis-client");
assert_eq!(info.name, "Redis 客户端");
```

#### 在 Tauri command 中使用

```rust
use worktools_shared_types::PluginInfo;

#[tauri::command]
async fn get_installed_plugins() -> Result<Vec<PluginInfo>, String> {
    // 返回所有已安装插件的元信息列表
    Ok(vec![plugin_info_a, plugin_info_b])
}
```

## 添加新字段

如需扩展 `PluginInfo`，在 `shared/types/src/lib.rs` 中添加新字段即可。由于使用了 `#[derive(Serialize, Deserialize)]`，JSON 序列化/反序列化会自动包含新字段。注意：新字段应设置合理的默认值或更新所有使用方。
