use serde::{Deserialize, Serialize};

/// 插件元信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub icon: String,
}

/// UI 字段类型定义
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UiField {
    #[serde(rename = "input")]
    Input {
        label: String,
        key: String,
        placeholder: Option<String>,
        default: Option<String>,
    },
    #[serde(rename = "number")]
    Number {
        label: String,
        key: String,
        default: Option<i32>,
        min: Option<i32>,
        max: Option<i32>,
    },
    #[serde(rename = "table")]
    Table {
        columns: Vec<String>,
        data_binding: String,
    },
    #[serde(rename = "button")]
    Button {
        label: String,
        key: String,
        action: String,
    },
    #[serde(rename = "checkbox")]
    Checkbox {
        label: String,
        key: String,
        default: Option<bool>,
    },
    #[serde(rename = "select")]
    Select {
        label: String,
        key: String,
        options: Vec<SelectOption>,
        default: Option<String>,
    },
}

/// 下拉选择选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
}

/// UI Schema
#[derive(Debug, Serialize, Deserialize)]
pub struct ViewSchema {
    pub fields: Vec<UiField>,
}

/// JSON-RPC 请求
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest<T> {
    pub jsonrpc: String,
    pub method: String,
    pub params: T,
    pub id: u64,
}

/// JSON-RPC 响应
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub result: Option<T>,
    pub error: Option<JsonRpcError>,
    pub id: u64,
}

/// JSON-RPC 错误
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// JSON-RPC 标准错误码
pub mod jsonrpc_error_code {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
}
