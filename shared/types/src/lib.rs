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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UiField {
    // === 基础组件 ===
    #[serde(rename = "input")]
    Input {
        label: String,
        key: String,
        placeholder: Option<String>,
        default: Option<String>,
        #[serde(rename = "input_type")]
        input_type: Option<String>, // "text", "password", "email", "url"
        required: Option<bool>,
    },
    #[serde(rename = "number")]
    Number {
        label: String,
        key: String,
        default: Option<i32>,
        min: Option<i32>,
        max: Option<i32>,
    },
    #[serde(rename = "button")]
    Button {
        label: String,
        key: String,
        action: Option<String>,
        icon: Option<String>,
        variant: Option<String>, // "primary", "secondary", "danger"
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
    #[serde(rename = "table")]
    Table {
        label: String,
        columns: Vec<String>,
        data_binding: String,
        actions: Option<Vec<TableAction>>,
    },

    // === 高级组件 (新增) ===

    /// 表格列表 (支持搜索、分页、批量操作)
    #[serde(rename = "table_list")]
    TableList {
        label: String,
        data_binding: String,
        columns: Vec<TableColumn>,
        actions: Vec<TableAction>,
        search_placeholder: Option<String>,
        pagination: Option<PaginationConfig>,
    },

    /// 表单容器 (支持嵌套、验证)
    #[serde(rename = "form")]
    Form {
        label: String,
        fields: Vec<UiField>,
        submit_action: String,
        cancel_action: Option<String>,
        validation: Option<FormValidation>,
    },

    /// 对话框
    #[serde(rename = "dialog")]
    Dialog {
        title: String,
        content: Vec<UiField>,
        trigger_action: String,
        width: Option<String>,
        height: Option<String>,
    },

    /// 标签页
    #[serde(rename = "tabs")]
    Tabs {
        tabs: Vec<TabItem>,
        default_tab: Option<String>,
    },

    /// 分组
    #[serde(rename = "group")]
    Group {
        label: String,
        fields: Vec<UiField>,
        collapsible: Option<bool>,
    },
}

/// 表格列定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableColumn {
    pub key: String,
    pub label: String,
    pub width: Option<String>,
    pub render: Option<String>, // "password", "icon", etc.
}

/// 表格操作按钮
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableAction {
    pub label: String,
    pub icon: String,
    pub action: String,
    pub confirm: Option<bool>, // 是否需要确认
}

/// 分页配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationConfig {
    pub page_size: usize,
    pub show_total: bool,
}

/// 表单验证规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub field: String,
    pub required: Option<bool>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
}

/// 表单验证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormValidation {
    pub rules: Vec<ValidationRule>,
}

/// 标签页项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabItem {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub content: Vec<UiField>,
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
