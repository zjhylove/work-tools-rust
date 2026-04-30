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

    fn handle_list_rules(&self) -> Result<Value> {
        let data = Self::load_data()?;
        Ok(serde_json::to_value(data)?)
    }

    fn handle_save_rule(&self, params: &Value) -> Result<Value> {
        let mut rule: RouteRule = serde_json::from_value(
            params
                .get("rule")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("缺少 rule 参数"))?,
        )?;

        let mut data = Self::load_data()?;

        if rule.id.is_empty() {
            rule.id = uuid::Uuid::new_v4().to_string();
            data.rules.push(rule.clone());
        } else {
            let idx = data
                .rules
                .iter()
                .position(|r| r.id == rule.id)
                .ok_or_else(|| anyhow::anyhow!("规则不存在"))?;
            data.rules[idx] = rule.clone();
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
        data.rules.retain(|r| r.id != id);
        Self::save_data(&data)?;
        Ok(serde_json::to_value(data)?)
    }

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

        let (database, table_suffix) = engine::execute_script(code, &rule)?;

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
        let matched: Vec<&RouteRule> = data
            .rules
            .iter()
            .filter(|r| match_rule(code, r))
            .collect();

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
                *existing = rule;
            } else {
                data.rules.push(rule);
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

fn match_rule(code: &str, rule: &RouteRule) -> bool {
    if rule.code_length > 0 && code.len() != rule.code_length as usize {
        return false;
    }
    if !rule.code_prefix.is_empty() {
        let prefixes: Vec<&str> = rule.code_prefix.split(',').map(|s| s.trim()).collect();
        if !prefixes.is_empty() && !prefixes.iter().any(|p| code.starts_with(p)) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rule(code_length: u32, code_prefix: &str) -> RouteRule {
        RouteRule {
            id: "test".to_string(),
            name: "test".to_string(),
            description: String::new(),
            code_length,
            code_prefix: code_prefix.to_string(),
            route_script: String::new(),
            tables: vec![],
        }
    }

    #[test]
    fn test_match_length() {
        let rule = make_rule(6, "");
        assert!(match_rule("123456", &rule));
        assert!(!match_rule("12345", &rule));
        assert!(!match_rule("1234567", &rule));
    }

    #[test]
    fn test_match_prefix() {
        let rule = make_rule(0, "ORD,LOG");
        assert!(match_rule("ORD123", &rule));
        assert!(match_rule("LOG456", &rule));
        assert!(!match_rule("ABC123", &rule));
    }

    #[test]
    fn test_match_any() {
        let rule = make_rule(0, "");
        assert!(match_rule("anything", &rule));
        assert!(match_rule("", &rule));
    }

    #[test]
    fn test_match_length_and_prefix() {
        let rule = make_rule(6, "ORD");
        assert!(match_rule("ORD123", &rule));
        assert!(!match_rule("ORD12", &rule));
        assert!(!match_rule("LOG123", &rule));
    }
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
