use serde::{Deserialize, Serialize};

/// Controller 信息（扫描结果）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerInfo {
    /// 类名 (如 com.example.controller.UserController)
    pub class_name: String,
    /// 类级别 @RequestMapping 路径
    pub class_path: String,
    /// 方法列表
    pub methods: Vec<MethodInfo>,
}

/// 方法信息（扫描结果）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    /// 方法名
    pub method_name: String,
    /// HTTP 方法 (GET/POST/PUT/DELETE/PATCH)
    pub http_method: String,
    /// 方法级别路径
    pub path: String,
    /// @ApiOperation 注解值
    pub api_name: String,
}

/// 完整 API 信息（解析结果）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    /// API 名称
    pub api_name: String,
    /// HTTP 方法
    pub http_method: String,
    /// 服务名称
    pub service_name: String,
    /// 业务模块 (从路径提取)
    pub business_module: String,
    /// 方法名
    pub method_name: String,
    /// 版本 (从路径提取，如 v1/v2)
    pub version: String,
    /// 完整路径
    pub full_path: String,
    /// 请求参数
    pub req_fields: Vec<ApiField>,
    /// 请求示例 JSON
    pub req_example: String,
    /// 响应节点
    pub resp_nodes: Vec<NodeInfo>,
    /// 响应示例 JSON
    pub resp_example: String,
}

/// API 字段信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiField {
    /// 字段名
    pub field_name: String,
    /// 字段类型
    pub field_type: String,
    /// 是否必填
    pub required: String,
    /// 字段长度
    pub field_length: String,
    /// 注释
    pub comment: String,
    /// 示例值
    pub example_value: String,
}

/// 响应节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// 节点名称
    pub node_name: String,
    /// 节点描述
    pub node_desc: String,
    /// 节点下的字段列表
    pub resp_fields: Vec<ApiField>,
}
