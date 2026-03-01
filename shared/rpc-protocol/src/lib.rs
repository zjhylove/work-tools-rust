use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use worktools_shared_types::{
    jsonrpc_error_code, JsonRpcError, JsonRpcRequest, JsonRpcResponse,
};

/// RPC 方法处理器类型
pub type RpcHandler = Box<dyn Fn(Value) -> Result<Value> + Send + Sync>;

/// JSON-RPC 服务器
pub struct RpcServer {
    handlers: HashMap<String, RpcHandler>,
}

impl RpcServer {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// 注册 RPC 方法处理器
    pub fn register_handler<F>(&mut self, method: &str, handler: F)
    where
        F: Fn(Value) -> Result<Value> + Send + Sync + 'static,
    {
        self.handlers.insert(method.to_string(), Box::new(handler));
    }

    /// 处理 JSON-RPC 请求
    pub fn handle(&self, req_str: &str) -> String {
        let req: JsonRpcRequest<Value> = match serde_json::from_str(req_str) {
            Ok(req) => req,
            Err(_) => {
                return error_response(0, jsonrpc_error_code::PARSE_ERROR, "Parse error");
            }
        };

        match self.handlers.get(&req.method) {
            Some(handler) => match handler(req.params) {
                Ok(result) => success_response(req.id, result),
                Err(e) => error_response(req.id, jsonrpc_error_code::INTERNAL_ERROR, &e.to_string()),
            },
            None => error_response(req.id, jsonrpc_error_code::METHOD_NOT_FOUND, "Method not found"),
        }
    }
}

impl Default for RpcServer {
    fn default() -> Self {
        Self::new()
    }
}

/// 构建成功响应
pub fn success_response<T>(id: u64, result: T) -> String
where
    T: serde::Serialize,
{
    let response = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(result),
        error: None,
        id,
    };
    serde_json::to_string(&response).unwrap_or_default()
}

/// 构建错误响应
pub fn error_response(id: u64, code: i32, message: &str) -> String {
    let response = JsonRpcResponse::<Value> {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.to_string(),
            data: None,
        }),
        id,
    };
    serde_json::to_string(&response).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_response() {
        let resp = success_response(1, "test");
        assert!(resp.contains("\"result\":\"test\""));
    }

    #[test]
    fn test_error_response() {
        let resp = error_response(1, -32601, "Method not found");
        assert!(resp.contains("\"error\""));
        assert!(resp.contains("-32601"));
    }
}
