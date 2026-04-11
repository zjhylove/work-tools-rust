use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub code_length: u32,
    pub code_prefix: String,
    pub route_script: String,
    #[serde(default)]
    pub tables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResult {
    pub database: String,
    pub tables: Vec<String>,
    pub code: String,
    pub rule_name: String,
    pub parse_time: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RouteData {
    #[serde(default)]
    pub rules: Vec<RouteRule>,
}
