//! # 数据库路由插件
//!
//! 根据业务编号解析目标数据库和表名。
//! 这是企业在分库分表场景下的核心工具。
//!
//! ## 应用场景
//! 大型系统的订单表常按编号分库分表：
//! - `ORD2024001` → 数据库 `db_order_2024`，表 `t_order_001`
//! - 路由规则可自定义，支持取模、日期、哈希等策略
//!
//! ## 核心能力
//! 1. **Rhai 脚本引擎**: 用户编写自定义路由逻辑
//! 2. **多种哈希算法**: FNV-1a、MurmurHash3、BKDR
//! 3. **一致性哈希**: 适合节点动态增减的分布式场景
//! 4. **规则模板**: 内置常用路由模式，快速上手
//!
//! ## Rust 知识点
//! - `retain`: Vec 的条件删除方法
//! - `iter().find()`: 查找第一个匹配的元素
//! - `map_err(|e| e.into())`: 在 Result 链中转换错误类型

use anyhow::Result;
use serde_json::Value;
use worktools_plugin_api::{storage::PluginStorage, Plugin};

mod engine;
mod model;

use model::{RouteData, RouteResult, RouteRule};

pub struct DbRouterPlugin;

impl DbRouterPlugin {
    fn storage() -> PluginStorage {
        PluginStorage::new("db-router", "db-router.json")
    }

    fn load_data() -> Result<RouteData> {
        Self::storage().load_json()
    }

    fn save_data(data: &RouteData) -> Result<()> {
        Self::storage().save_json(data)
    }

    // ── 规则 CRUD ──

    fn handle_list_rules(&self) -> Result<Value> {
        let data = Self::load_data()?;
        Ok(serde_json::to_value(data)?)
    }

    fn handle_save_rule(&self, params: &Value) -> Result<Value> {
        // 从前端传来的 JSON 中反序列化出 RouteRule
        let mut rule: RouteRule = serde_json::from_value(
            params
                .get("rule")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("缺少 rule 参数"))?,
        )?;

        let mut data = Self::load_data()?;

        if rule.id.is_empty() {
            // 无 ID = 新增
            rule.id = uuid::Uuid::new_v4().to_string();
            data.rules.push(rule.clone());
            tracing::info!(name = %rule.name, id = %rule.id, "新建路由规则");
        } else {
            // 有 ID = 更新（原地替换）
            let idx = data
                .rules
                .iter()
                .position(|r| r.id == rule.id)
                .ok_or_else(|| anyhow::anyhow!("规则不存在"))?;
            data.rules[idx] = rule.clone();
            tracing::info!(name = %rule.name, id = %rule.id, "更新路由规则");
        }

        Self::save_data(&data)?;
        Ok(serde_json::to_value(data)?)
    }

    fn handle_delete_rule(&self, params: &Value) -> Result<Value> {
        let id = params
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 id 参数"))?;

        let mut data = Self::load_data()?;
        // `retain` 保留满足条件的元素（这里是"ID 不等于目标"）
        // 等价于 Java: list.removeIf(r -> r.id == id)
        data.rules.retain(|r| r.id != id);
        Self::save_data(&data)?;

        Ok(serde_json::to_value(data)?)
    }

    // ── 路由解析 ──

    fn handle_parse_route(&self, params: &Value) -> Result<Value> {
        let code = params
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 code 参数"))?;
        let rule_id = params
            .get("rule_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 rule_id 参数"))?;

        let data = Self::load_data()?;
        let rule = data
            .rules
            .iter()
            .find(|r| r.id == rule_id)
            .ok_or_else(|| anyhow::anyhow!("规则不存在"))?
            .clone();

        // 执行 Rhai 脚本解析路由
        let (database, table_suffix) = engine::execute_script(code, &rule)?;

        tracing::info!(%database, %table_suffix, rule = %rule.name, "路由解析完成");

        // 生成完整的表名列表
        let tables: Vec<String> = if rule.tables.is_empty() {
            vec![format!("table{table_suffix}")]
        } else {
            rule.tables
                .iter()
                .map(|t| format!("{t}{table_suffix}"))
                .collect()
        };

        let result = RouteResult {
            database,
            tables,
            code: code.to_string(),
            rule_name: rule.name.clone(),
            parse_time: chrono::Utc::now().to_rfc3339(),
        };

        Ok(serde_json::to_value(result)?)
    }

    fn handle_match_rules(&self, params: &Value) -> Result<Value> {
        let code = params
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 code 参数"))?;

        let data = Self::load_data()?;
        // 筛选匹配的规则（按长度和前缀）
        let matched: Vec<&RouteRule> = data.rules.iter().filter(|r| match_rule(code, r)).collect();

        Ok(serde_json::to_value(matched)?)
    }

    fn handle_import_rules(&self, params: &Value) -> Result<Value> {
        let imported_rules: Vec<RouteRule> = serde_json::from_value(
            params
                .get("rules")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("缺少 rules 参数"))?,
        )?;

        let mut data = Self::load_data()?;
        for rule in imported_rules {
            if let Some(existing) = data.rules.iter_mut().find(|r| r.id == rule.id) {
                *existing = rule; // 替换已存在的
            } else {
                data.rules.push(rule); // 追加新的
            }
        }

        Self::save_data(&data)?;
        Ok(serde_json::to_value(data)?)
    }

    fn handle_export_rules(&self) -> Result<Value> {
        let data = Self::load_data()?;
        Ok(serde_json::to_value(&data.rules)?)
    }

    fn handle_get_templates(&self) -> Result<Value> {
        let templates = engine::get_templates();
        Ok(serde_json::to_value(templates)?)
    }
}

/// 检查编号是否匹配路由规则（长度限制 + 前缀匹配）
fn match_rule(code: &str, rule: &RouteRule) -> bool {
    // 长度检查：0 表示不限制
    if rule.code_length > 0 && code.len() != rule.code_length as usize {
        return false;
    }
    // 前缀检查：逗号分隔多个前缀，匹配任一即可
    if !rule.code_prefix.is_empty() {
        let prefixes: Vec<&str> = rule.code_prefix.split(',').map(|s| s.trim()).collect();
        if !prefixes.is_empty() && !prefixes.iter().any(|p| code.starts_with(p)) {
            return false;
        }
    }
    true
}

impl Plugin for DbRouterPlugin {
    fn id(&self) -> &str {
        "db-router"
    }
    fn name(&self) -> &str {
        "数据库路由"
    }
    fn description(&self) -> &str {
        "根据编号解析数据库和表路由规则，支持多表关联"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn icon(&self) -> &str {
        "🗄️"
    }
    fn get_view(&self) -> String {
        "<div>插件前端资源加载中...</div>".to_string()
    }

    fn handle_call(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let result = match method {
            "list_rules" => self.handle_list_rules(),
            "save_rule" => self.handle_save_rule(&params),
            "delete_rule" => self.handle_delete_rule(&params),
            "parse_route" => self.handle_parse_route(&params),
            "match_rules" => self.handle_match_rules(&params),
            "import_rules" => self.handle_import_rules(&params),
            "export_rules" => self.handle_export_rules(),
            "get_templates" => self.handle_get_templates(),
            _ => Err(anyhow::anyhow!("未知方法: {method}")),
        };
        result.map_err(|e| e.into())
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(DbRouterPlugin));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_rule(prefix: &str, length: u32) -> RouteRule {
        RouteRule {
            id: "test".to_string(),
            name: "test".to_string(),
            description: String::new(),
            code_prefix: prefix.to_string(),
            code_length: length,
            route_script: String::new(),
            tables: Vec::new(),
        }
    }

    #[test]
    fn test_plugin_info() {
        let plugin = DbRouterPlugin;
        assert_eq!(plugin.id(), "db-router");
        assert_eq!(plugin.name(), "数据库路由");
        assert_eq!(plugin.version(), "1.0.0");
        assert!(!plugin.icon().is_empty());
        assert!(!plugin.get_view().is_empty());
    }

    #[test]
    fn test_unknown_method_returns_error() {
        let mut plugin = DbRouterPlugin;
        let result = plugin.handle_call("nonexistent", json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn test_match_rule_prefix() {
        let rule = make_rule("ORD", 0);
        assert!(match_rule("ORD2024001", &rule));
        assert!(!match_rule("INV2024001", &rule));
    }

    #[test]
    fn test_match_rule_length() {
        let rule = make_rule("", 10);
        assert!(match_rule("1234567890", &rule));
        assert!(!match_rule("12345", &rule));
    }

    #[test]
    fn test_match_rule_multi_prefix() {
        let rule = make_rule("ORD,INV", 0);
        assert!(match_rule("ORD001", &rule));
        assert!(match_rule("INV001", &rule));
        assert!(!match_rule("PAY001", &rule));
    }

    #[test]
    fn test_match_rule_prefix_and_length() {
        let rule = make_rule("ORD", 10);
        assert!(match_rule("ORD1234567", &rule));
        assert!(!match_rule("ORD123", &rule));
        assert!(!match_rule("INV1234567", &rule));
    }
    #[test]
    fn test_hash_code_java_compatible() {
        // 复现 Java Math.abs(account_no.hashCode()) % 100 的分库路由场景。
        // Java String.hashCode() 返回 int（32 位有符号，乘 31 累加，溢出截断），
        // 对 "ECCC8E65524D2487E043650A1803172F" 算得 1974971894 → abs % 100 = 94。
        // 旧实现误用 i64 不溢出，结果为 86，导致路由到错误分库。
        let mut rule = make_rule("", 0);
        rule.route_script =
            "let database = (abs(hash_code(code)) % 100).to_string(); let table_suffix = \"\";"
                .to_string();
        let (database, _) =
            crate::engine::execute_script("ECCC8E65524D2487E043650A1803172F", &rule).unwrap();
        assert_eq!(database, "94");
    }
}
