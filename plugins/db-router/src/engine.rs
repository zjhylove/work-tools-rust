use anyhow::{anyhow, Result};
use rhai::{Engine, Scope};

use super::model::RouteRule;

pub fn execute_script(code: &str, rule: &RouteRule) -> Result<(String, String)> {
    let mut engine = Engine::new();
    engine.set_max_operations(100_000);
    engine.set_max_modules(0);
    engine.set_max_call_levels(32);

    let mut scope = Scope::new();
    scope.push("code", code.to_string());

    engine
        .run_with_scope(&mut scope, &rule.route_script)
        .map_err(|e| anyhow!("脚本执行错误: {e}"))?;

    let database = scope
        .get_value::<String>("database")
        .ok_or_else(|| anyhow!("脚本未设置 database 变量"))?;

    let table_suffix = scope
        .get_value::<String>("table_suffix")
        .ok_or_else(|| anyhow!("脚本未设置 table_suffix 变量"))?;

    Ok((database, table_suffix))
}

pub fn get_templates() -> Vec<RouteRule> {
    vec![
        RouteRule {
            id: String::new(),
            name: "按位置截取".to_string(),
            description: "根据编号的固定位置截取库名和表名后缀".to_string(),
            code_length: 0,
            code_prefix: String::new(),
            route_script: r#"let database = "db_order_" + code[3..7];
let table_suffix = "_" + code[15..18];"#.to_string(),
            tables: vec![],
        },
        RouteRule {
            id: String::new(),
            name: "取模分片".to_string(),
            description: "根据编号长度对分片数取模".to_string(),
            code_length: 0,
            code_prefix: String::new(),
            route_script: r#"let n = code.len();
let shard = (n % 16).to_string();
let database = "db_order_" + shard;
let table_suffix = "_" + shard;"#.to_string(),
            tables: vec![],
        },
        RouteRule {
            id: String::new(),
            name: "日期分表".to_string(),
            description: "从编号中提取日期部分路由到对应月份表".to_string(),
            code_length: 0,
            code_prefix: String::new(),
            route_script: r#"let year = code[3..7];
let month = code[7..9];
let database = "db_log";
let table_suffix = "_" + year + "_" + month;"#.to_string(),
            tables: vec![],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rule(script: &str) -> RouteRule {
        RouteRule {
            id: "test".to_string(),
            name: "test".to_string(),
            description: String::new(),
            code_length: 0,
            code_prefix: String::new(),
            route_script: script.to_string(),
            tables: vec!["t_order".to_string(), "t_order_item".to_string()],
        }
    }

    #[test]
    fn test_execute_positional() {
        let rule = make_rule(r#"let database = "db_" + code[3..7];
let table_suffix = "_" + code[7..10];"#);
        let (db, suffix) = execute_script("ORD2024001", &rule).unwrap();
        assert_eq!(db, "db_2024");
        assert_eq!(suffix, "_001");
    }

    #[test]
    fn test_execute_modulo() {
        let rule = make_rule(r#"let n = code.len();
let shard = (n % 4).to_string();
let database = "db_" + shard;
let table_suffix = "_" + shard;"#);
        let (db, suffix) = execute_script("ABC123", &rule).unwrap();
        assert_eq!(db, "db_2");
        assert_eq!(suffix, "_2");
    }

    #[test]
    fn test_execute_missing_variable() {
        let rule = make_rule("let database = \"db\";");
        let result = execute_script("test", &rule);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("table_suffix"));
    }

    #[test]
    fn test_execute_script_error() {
        let rule = make_rule("let x = 1 / 0;");
        let result = execute_script("test", &rule);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_templates() {
        let templates = get_templates();
        assert_eq!(templates.len(), 3);
        assert_eq!(templates[0].name, "按位置截取");
        assert_eq!(templates[1].name, "取模分片");
        assert_eq!(templates[2].name, "日期分表");
    }
}
