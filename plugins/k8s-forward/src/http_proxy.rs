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

pub struct HttpProxySvc {
    port: u16,
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

    pub fn register(&self, domain: &str, target: &str, rule_id: &str, editable: bool) {
        self.mappings.lock().unwrap().insert(domain.to_string(), target.to_string());
        self.mapping_list.lock().unwrap().push(ProxyMapping {
            domain: domain.to_string(),
            target: target.to_string(),
            rule_id: rule_id.to_string(),
            editable,
        });
    }

    pub fn unregister(&self, domain: &str) {
        self.mappings.lock().unwrap().remove(domain);
        self.mapping_list.lock().unwrap().retain(|m| m.domain != domain);
    }

    pub fn unregister_by_rule_id(&self, rule_id: &str) {
        let domains: Vec<String> = self.mapping_list.lock().unwrap()
            .iter()
            .filter(|m| m.rule_id == rule_id)
            .map(|m| m.domain.clone())
            .collect();
        for d in domains {
            self.unregister(&d);
        }
    }

    pub fn list_mappings(&self) -> Vec<ProxyMapping> {
        self.mapping_list.lock().unwrap().clone()
    }

    /// 更新可编辑(editable)的映射项（即 pod 地址，而非别名）
    pub fn update_mapping(&self, rule_id: &str, new_domain: &str) -> Result<ProxyMapping> {
        let mut list = self.mapping_list.lock().unwrap();
        if let Some(m) = list.iter_mut().find(|m| m.rule_id == rule_id && m.editable) {
            let old_domain = m.domain.clone();
            let target = m.target.clone();
            let editable = m.editable;
            self.mappings.lock().unwrap().remove(&old_domain);
            m.domain = new_domain.to_string();
            self.mappings.lock().unwrap().insert(new_domain.to_string(), target);
            return Ok(ProxyMapping {
                domain: new_domain.to_string(),
                target: m.target.clone(),
                rule_id: rule_id.to_string(),
                editable,
            });
        }
        Err(anyhow::anyhow!("未找到 rule_id 对应的映射"))
    }

    pub async fn start(&mut self) -> Result<()> {
        let port = self.port;
        let mappings = self.mappings.clone();

        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
        let (tx, rx) = oneshot::channel::<()>();
        self.shutdown_tx = Some(tx);
        self.running = true;

        tokio::spawn(async move {
            let mut graceful = std::pin::pin!(async move { rx.await.ok() });

            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, _)) => {
                                let io = TokioIo::new(stream);
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
                    _ = &mut graceful => break,
                }
            }
        });

        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.running = false;
    }
}

async fn proxy_request(
    req: Request<Incoming>,
    mappings: Arc<Mutex<HashMap<String, String>>>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let host = req.headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let host_without_port = host.split(':').next().unwrap_or(host);
    let uri_addr = req.uri().host().map(|h| {
        let port = req.uri().port_u16().unwrap_or(80);
        format!("{}:{}", h, port)
    });

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

async fn forward_request(
    req: Request<Incoming>,
    target: &str,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build_http();

    let (parts, body) = req.into_parts();
    let body_bytes = body.collect().await?.to_bytes();

    let path = parts.uri.path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");

    let uri = format!("http://{}{}", target, path);
    let uri: hyper::Uri = uri.parse()?;

    let mut builder = Request::builder()
        .method(parts.method)
        .uri(&uri);

    for (key, value) in parts.headers.iter() {
        if key.as_str().to_lowercase() != "host" {
            builder = builder.header(key, value);
        }
    }
    builder = builder.header("Host", target);

    let proxy_req = builder.body(Full::new(body_bytes))?;

    let resp = client.request(proxy_req).await?;

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
