# 插件开发快速参考指南

## 使用新的共享 API

### 1. 数据存储 (PluginStorage)

```rust
use worktools_plugin_api::storage::PluginStorage;

// 创建存储实例
fn storage() -> PluginStorage {
    PluginStorage::new("plugin-id", "data.json")
}

// 加载数据
fn load_data() -> Result<MyData> {
    Self::storage().load_json()
}

// 保存数据
fn save_data(data: &MyData) -> Result<()> {
    Self::storage().save_json(data)
}

// 保存数据并保留特定字段
fn save_with_preservation(data: &MyData) -> Result<()> {
    Self::storage().save_json_preserving(data, &["field1", "field2"])
}
```

### 2. 错误处理 (PluginError)

```rust
use worktools_plugin_api::{PluginError, PluginResult, method_error, param_error};

// 使用 PluginResult
fn my_method(&mut self, params: Value) -> PluginResult<Value> {
    // 检查参数
    let param = params.get("param")
        .and_then(|v| v.as_str())
        .ok_or_else(|| param_error!("缺少 param 参数"))?;

    // 返回错误
    if some_error {
        return Err(method_error!("my_method", "操作失败"));
    }

    Ok(serde_json::json!({ "result": "success" }))
}
```

## 最佳实践

### ✅ 推荐做法

1. **使用 derive 宏**
   ```rust
   #[derive(Debug, Default, Serialize, Deserialize)]
   struct MyData {
       entries: Vec<Entry>,
   }
   ```

2. **使用标准库方法**
   ```rust
   // ✅ 好
   if value.is_multiple_of(16) { }

   // ❌ 差
   if value % 16 == 0 { }
   ```

3. **使用迭代器**
   ```rust
   // ✅ 好
   for item in &slice[start..] { }

   // ❌ 差
   for i in start..slice.len() {
       let item = &slice[i];
   }
   ```

4. **使用 or 而非 or_else**
   ```rust
   // ✅ 好
   value1.or(value2)

   // ❌ 差
   value1.or_else(|| value2)
   ```

### ❌ 避免的做法

1. **不要手动实现可 derive 的 trait**
   ```rust
   // ❌ 差
   impl Default for MyData {
       fn default() -> Self { Self { entries: Vec::new() } }
   }

   // ✅ 好
   #[derive(Default)]
   struct MyData { entries: Vec<Entry> }
   ```

2. **不要重复代码**
   ```rust
   // ❌ 差 - 在每个插件中重复
   fn get_data_path() -> Result<PathBuf> {
       // 50+ 行路径管理代码
   }

   // ✅ 好 - 使用共享存储
   fn storage() -> PluginStorage {
       PluginStorage::new("plugin-id", "data.json")
   }
   ```

3. **不要使用字符串作为错误**
   ```rust
   // ❌ 差
   fn do_something() -> Result<Value, String> {
       Err("error message".to_string())
   }

   // ✅ 好
   fn do_something() -> PluginResult<Value> {
       Err(PluginError::InvalidParameter("error message".to_string()))
   }
   ```

## 常用宏

### method_error!
```rust
// 简单形式
method_error!("method_name", "error message")

// 带格式化
method_error!("method_name", "error: {}", detail)
```

### param_error!
```rust
// 简单形式
param_error!("missing parameter")

// 带格式化
param_error!("invalid {}: {}", param_name, value)
```

## 测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_load() {
        let storage = PluginStorage::new("test", "test.json");

        // 保存
        let data = TestData { entries: vec![] };
        storage.save_json(&data).unwrap();

        // 加载
        let loaded: TestData = storage.load_json().unwrap();
        assert_eq!(loaded.entries.len(), 0);
    }
}
```

## 更多信息

- 详细优化总结: [docs/OPTIMIZATION_SUMMARY.md](OPTIMIZATION_SUMMARY.md)
- 插件 API 文档: [shared/plugin-api/src/lib.rs](../shared/plugin-api/src/lib.rs)
