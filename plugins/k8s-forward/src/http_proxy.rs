//! # HTTP 反向代理服务
//!
//! 基于 hyper (HTTP/1.1) 实现的轻量级反向代理。
//! 将收到的 HTTP 请求根据域名转发到对应的本地端口。
//!
//! ## 工作原理
//! ```
//! 浏览器 → localhost:80 (代理)
//!   → 检查 Host 头，查找目标映射
//!   → 转发到 localhost:10001 (SSH 隧道)
//!   → 返回响应给浏览器
//! ```
//!
//! ## Rust 知识点
//! - `hyper`: 高性能 HTTP 库（tokio 生态）
//! - `service_fn`: 将闭包转为 HTTP 服务
//! - `tokio::select!`: 同时等待多个异步操作（accept + shutdown）
//! - `oneshot::channel`: 一次性通知通道（用于优雅关闭）
//! - `Arc<Mutex<HashMap>>`: 多任务共享的域名映射表

use anyhow::Result;
use hyper::{body::Incoming, server::conn::http1, service::service_fn, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use http_body_util::{Full, BodyExt};
use hyper::body::Bytes;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::models::ProxyMapping;

/// HTTP 反向代理服务
///
/// ## 字段说明
/// - `mappings`: 域名 → 目标地址的映射表
/// - `mapping_list`: 映射列表（用于 CRUD 操作）
/// - `shutdown_tx`: 优雅关闭的发送端（oneshot channel）
pub struct HttpProxySvc {
    port: u16,
    /// 域名 → 目标地址 的映射（如 "pod-nginx" → "127.0.0.1:10001"）
    mappings: Arc<Mutex<HashMap<String, String>>>,
    mapping_list: Arc<Mutex<Vec<ProxyMapping>>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    running: bool,
}

impl HttpProxySvc {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            mappings: Arc::new(Mutex::new(HashMap::new())),
            mapping_list: Arc::new(Mutex::new(vec![])),
            shutdown_tx: None,
            running: false,
        }
    }

    pub fn is_running(&self) -> bool { self.running }
    pub fn port(&self) -> u16 { self.port }

    /// 注册域名映射
    /// `editable` 参数: Pod 地址映射为 true（允许用户修改域名），别名为 false
    pub fn register(&self, domain: &str, target: &str, rule_id: &str, editable: bool) {
        self.mappings.lock().unwrap().insert(domain.to_string(), target.to_string());
        self.mapping_list.lock().unwrap().push(ProxyMapping {
            domain: domain.to_string(), target: target.to_string(),
            rule_id: rule_id.to_string(), editable,
        });
    }

    /// 注销单个域名映射
    pub fn unregister(&self, domain: &str) {
        self.mappings.lock().unwrap().remove(domain);
        self.mapping_list.lock().unwrap().retain(|m| m.domain != domain);
    }

    /// 按规则 ID 注销所有相关映射
    pub fn unregister_by_rule_id(&self, rule_id: &str) {
        // 先收集要删除的域名（避免迭代中修改）
        let domains: Vec<String> = self.mapping_list.lock().unwrap()
            .iter().filter(|m| m.rule_id == rule_id)
            .map(|m| m.domain.clone()).collect();
        for d in domains { self.unregister(&d); }
    }

    pub fn list_mappings(&self) -> Vec<ProxyMapping> {
        self.mapping_list.lock().unwrap().clone()
    }

    /// 更新可编辑的映射（修改 Pod 地址的域名）
    pub fn update_mapping(&self, rule_id: &str, new_domain: &str) -> Result<ProxyMapping> {
        let mut list = self.mapping_list.lock().unwrap();
        if let Some(m) = list.iter_mut().find(|m| m.rule_id == rule_id && m.editable) {
            let old_domain = m.domain.clone();
            self.mappings.lock().unwrap().remove(&old_domain);
            m.domain = new_domain.to_string();
            self.mappings.lock().unwrap().insert(new_domain.to_string(), m.target.clone());
            return Ok(ProxyMapping {
                domain: new_domain.to_string(), target: m.target.clone(),
                rule_id: rule_id.to_string(), editable: m.editable,
            });
        }
        Err(anyhow::anyhow!("未找到 rule_id 对应的映射"))
    }

    /// 启动 HTTP 代理服务器
    ///
    /// ## Rust 知识点: tokio::select!
    /// `tokio::select!` 同时等待多个异步 Future，哪个先完成就执行哪个分支。
    /// 这里用于：同时等待新连接 和 关闭信号。
    ///
    /// ## 优雅关闭
    /// 使用 `oneshot::channel` 实现：`stop()` 发送信号 → `select!` 捕获 → 退出 accept 循环。
    pub async fn start(&mut self) -> Result<()> {
        let port = self.port;
        let mappings = self.mappings.clone();

        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;

        // 创建 oneshot channel 用于优雅关闭
        let (tx, rx) = oneshot::channel::<()>();
        self.shutdown_tx = Some(tx);
        self.running = true;

        // 启动主循环任务
        tokio::spawn(async move {
            // `std::pin::pin!` 固定 Future（oneshot 的接收端）
            let mut graceful = std::pin::pin!(async move { rx.await.ok() });

            loop {
                tokio::select! {
                    // 分支1: 接受新连接
                    result = listener.accept() => {
                        match result {
                            Ok((stream, _)) => {
                                let io = TokioIo::new(stream); // 标准 TCP 流 → Tokio 异步流
                                let mappings = mappings.clone();
                                tokio::spawn(async move {
                                    let svc = service_fn(move |req| {
                                        proxy_request(req, mappings.clone())
                                    });
                                    if let Err(e) = http1::Builder::new()
                                        .serve_connection(io, svc)
                                        .await
                                    {
                                        tracing::error!("代理连接错误: {}", e);
                                    }
                                });
                            }
                            Err(e) => tracing::error!("Accept error: {}", e),
                        }
                    }
                    // 分支2: 收到关闭信号
                    _ = &mut graceful => break,
                }
            }
        });

        Ok(())
    }

    /// 停止代理服务器
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()); // 发送关闭信号
        }
        self.running = false;
    }
}

/// 代理请求处理函数
///
/// ## 域名解析优先级
/// 1. Host 头（含端口）：如 "my-pod:80"
/// 2. Host 头（不含端口）：如 "my-pod"
/// 3. URI 中的地址：如请求 "http://10.0.0.1:8080/path"
async fn proxy_request(
    req: Request<Incoming>,
    mappings: Arc<Mutex<HashMap<String, String>>>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    // 提取多个可能的 host 来源
    let host = req.headers().get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let host_without_port = host.split(':').next().unwrap_or(host);

    let uri_addr = req.uri().host().map(|h| {
        let port = req.uri().port_u16().unwrap_or(80);
        format!("{}:{}", h, port)
    });

    // 按优先级查找映射
    let target = {
        let map = mappings.lock().unwrap();
        map.get(host).cloned()
            .or_else(|| map.get(host_without_port).cloned())
            .or_else(|| uri_addr.as_ref().and_then(|a| map.get(a).cloned()))
    };

    if let Some(ref t) = target {
        forward_or_502(req, t).await
    } else if let Some(ref t) = uri_addr {
        forward_or_502(req, t).await
    } else {
        let mut resp = Response::new(Full::new(Bytes::from("未找到目标地址")));
        *resp.status_mut() = StatusCode::NOT_FOUND;
        Ok(resp)
    }
}

/// 转发请求，失败时返回 502 Bad Gateway
async fn forward_or_502(
    req: Request<Incoming>,
    target: &str,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    match forward_request(req, target).await {
        Ok(resp) => Ok(resp),
        Err(e) => {
            tracing::error!("[proxy] 转发失败: {} - {}", target, e);
            let mut resp = Response::new(Full::new(Bytes::from(
                format!("转发目标不可达: {}", target)
            )));
            *resp.status_mut() = StatusCode::BAD_GATEWAY;
            Ok(resp)
        }
    }
}

/// 实际执行 HTTP 转发
///
/// 将请求的 path/query/headers/body 原封不动转发到目标地址，
/// 然后返回目标的响应。
async fn forward_request(
    req: Request<Incoming>,
    target: &str,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    // 创建 HTTP 客户端
    let client = hyper_util::client::legacy::Client::builder(
        hyper_util::rt::TokioExecutor::new()
    ).build_http();

    // 分解请求
    let (parts, body) = req.into_parts();
    let body_bytes = body.collect().await?.to_bytes();

    // 构建目标 URI
    let path = parts.uri.path_and_query()
        .map(|pq| pq.as_str()).unwrap_or("/");
    let uri = format!("http://{}{}", target, path);
    let uri: hyper::Uri = uri.parse()?;

    // 构建转发请求（复制 method + headers + body）
    let mut builder = Request::builder()
        .method(parts.method)
        .uri(&uri);

    for (key, value) in parts.headers.iter() {
        if key.as_str().to_lowercase() != "host" {
            builder = builder.header(key, value);
        }
    }
    builder = builder.header("Host", target); // 设置正确的 Host 头

    let proxy_req = builder.body(Full::new(body_bytes))?;

    // 发送请求
    let resp = client.request(proxy_req).await?;

    // 构建响应
    let (resp_parts, resp_body) = resp.into_parts();
    let resp_body_bytes = resp_body.collect().await?.to_bytes();

    let mut response = Response::new(Full::new(resp_body_bytes));
    *response.status_mut() = resp_parts.status;
    *response.version_mut() = resp_parts.version;
    for (key, value) in resp_parts.headers.iter() {
        response.headers_mut().insert(key, value.clone());
    }

    Ok(response)
}
