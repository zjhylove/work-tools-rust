use crate::models::*;
use anyhow::{anyhow, Result};
use reqwest::redirect::Policy;
use reqwest::{cookie::Jar, Client};
use std::sync::Arc;

pub struct KuboardClient {
    client: Client,
    base_url: String,
    logged_in: bool,
    username: String,
    password: String,
    kuboard_token: Option<String>,
}

impl KuboardClient {
    pub fn new(base_url: &str) -> Self {
        let cookie_jar = Arc::new(Jar::default());
        let client = Client::builder()
            .cookie_provider(cookie_jar)
            .redirect(Policy::none())
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            logged_in: false,
            username: String::new(),
            password: String::new(),
            kuboard_token: None,
        }
    }

    fn url(&self, path: &str) -> String {
        if path.starts_with("http") {
            path.to_string()
        } else {
            format!("{}{}", self.base_url, path)
        }
    }

    fn resolve_url(base: &str, location: &str) -> String {
        if location.starts_with("http") {
            location.to_string()
        } else if location.starts_with('/') {
            if let Some(pos) = base.find("://") {
                let after = &base[pos + 3..];
                if let Some(he) = after.find('/') {
                    format!("{}{}", &base[..pos + 3 + he], location)
                } else {
                    format!("{}{}", base, location)
                }
            } else {
                format!("{}{}", base, location)
            }
        } else {
            format!("{}/{}", base.trim_end_matches('/'), location)
        }
    }

    fn capture_token(resp: &reqwest::Response, token: &mut Option<String>) {
        for (name, value) in resp.headers() {
            if name.as_str().eq_ignore_ascii_case("set-cookie") {
                let v = value.to_str().unwrap_or("");
                if let Some(tok) = v.strip_prefix("KuboardToken=") {
                    let end = tok.find(';').unwrap_or(tok.len());
                    *token = Some(tok[..end].to_string());
                }
            }
        }
    }

    /// 手动跟随 GET redirect 链，同时捕获 KuboardToken cookie
    async fn follow_get_redirects(
        client: &Client,
        base_url: &str,
        mut resp: reqwest::Response,
        token: &mut Option<String>,
    ) -> Result<reqwest::Response> {
        Self::capture_token(&resp, token);
        loop {
            let status = resp.status().as_u16();
            if !(300..400).contains(&status) {
                return Ok(resp);
            }
            let location = resp
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| anyhow!("Redirect (HTTP {}) 缺少 Location header", status))?
                .to_string();
            let _ = resp.bytes().await;
            let next_url = Self::resolve_url(base_url, &location);
            resp = client.get(&next_url).send().await?;
            Self::capture_token(&resp, token);
        }
    }

    /// 获取 SSO req_id
    async fn fetch_req_id(&self) -> Result<String> {
        let resp = self
            .client
            .get(&self.url("/kuboard/cluster"))
            .send()
            .await?;
        let mut dummy = None;
        let resp =
            Self::follow_get_redirects(&self.client, &self.base_url, resp, &mut dummy).await?;
        let final_url = resp.url().to_string();
        if let Some(pos) = final_url.find("req=") {
            return Ok(final_url[pos + 4..].to_string());
        }
        let resp = self
            .client
            .get(&self.url("/login?state=%2Fkuboard%2Fcluster"))
            .send()
            .await?;
        let resp =
            Self::follow_get_redirects(&self.client, &self.base_url, resp, &mut dummy).await?;
        let final_url = resp.url().to_string();
        if let Some(pos) = final_url.find("req=") {
            return Ok(final_url[pos + 4..].to_string());
        }
        Err(anyhow!("无法获取 SSO req_id，请检查 Kuboard 地址"))
    }

    /// SSO 登录
    pub async fn login(&mut self, username: &str, password: &str) -> Result<LoginResult> {
        self.username = username.to_string();
        self.password = password.to_string();

        let req_id = self.fetch_req_id().await?;

        let pwd_json = format!("{{\"password\":\"{}\"}}", password);
        let body = format!(
            "login={}&password={}",
            urlencoding(username),
            urlencoding(&pwd_json)
        );

        let post_url = self.url(&format!("/sso/auth/default?req={}", req_id));
        let resp = self
            .client
            .post(&post_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?;

        let status = resp.status().as_u16();

        if (300..400).contains(&status) {
            if let Some(location) = resp.headers().get("location").and_then(|v| v.to_str().ok()) {
                let next_url = Self::resolve_url(&self.base_url, location);
                let next_resp = self.client.get(&next_url).send().await?;
                Self::capture_token(&next_resp, &mut self.kuboard_token);
                let _ = Self::follow_get_redirects(
                    &self.client,
                    &self.base_url,
                    next_resp,
                    &mut self.kuboard_token,
                )
                .await?;
                if self.kuboard_token.is_some() {
                    self.logged_in = true;
                    return Ok(LoginResult {
                        success: true,
                        mfa_required: None,
                        message: None,
                    });
                }
            }
        }

        let text = resp.text().await?;
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(msg) = json["message"].as_str() {
                return Ok(parse_login_message(msg));
            }
        }

        if status == 200 {
            self.logged_in = true;
            return Ok(LoginResult {
                success: true,
                mfa_required: None,
                message: None,
            });
        }

        Err(anyhow!(
            "登录失败: HTTP {} - {}",
            status,
            text.chars().take(200).collect::<String>()
        ))
    }

    /// MFA 验证
    pub async fn mfa_verify(&mut self, passcode: &str) -> Result<()> {
        let resp = self
            .client
            .post(&self.url("/login/password"))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "username": self.username,
                "password": self.password,
                "passcode": passcode,
            }))
            .send()
            .await?;

        let text = resp.text().await?;
        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|_| anyhow!("MFA 响应非 JSON"))?;
        let mfa_status = json["mfaVerifyStatus"].as_str().unwrap_or("");
        match mfa_status {
            "Pass" | "Restored" => {
                self.logged_in = true;
                Ok(())
            }
            "Block" => Err(anyhow!("MFA 验证失败，验证码错误")),
            _ => Err(anyhow!("MFA 验证失败: {}", mfa_status)),
        }
    }

    pub fn is_logged_in(&self) -> bool {
        self.logged_in
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    /// 为 API 请求添加认证 Cookie
    fn api_req(&self, path: &str) -> reqwest::RequestBuilder {
        let mut req = self.client.get(&self.url(path));
        if let Some(ref token) = self.kuboard_token {
            req = req.header(
                "Cookie",
                format!("KuboardToken={}; KuboardLogin=true", token),
            );
        }
        req
    }

    pub async fn list_clusters(&self) -> Result<Vec<String>> {
        let resp = self
            .api_req("/kuboard-api/cluster/GLOBAL/kind/KuboardLicensedClusters/LicensedClusters")
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        let json: serde_json::Value = serde_json::from_str(&text).map_err(|_| {
            anyhow!(
                "集群列表接口返回非 JSON 数据 (HTTP {}, body: {})",
                status.as_u16(),
                &text[..text.len().min(500)]
            )
        })?;
        let clusters: Vec<String> = json["spec"]["clusters"]
            .as_object()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        Ok(clusters)
    }

    pub async fn list_namespaces(&self, cluster: &str) -> Result<Vec<String>> {
        let path = format!("/k8s-api/{}/api/v1/namespaces", cluster);
        let resp = self.api_req(&path).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        let json: serde_json::Value = serde_json::from_str(&text).map_err(|_| {
            anyhow!(
                "命名空间列表接口返回非 JSON 数据 (HTTP {}, body: {})",
                status.as_u16(),
                &text[..text.len().min(500)]
            )
        })?;
        let namespaces: Vec<String> = json["items"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|ns| ns["metadata"]["name"].as_str().map(String::from))
            .collect();
        Ok(namespaces)
    }

    pub async fn list_pods(&self, cluster: &str, namespace: &str) -> Result<Vec<PodInfo>> {
        let path = format!("/k8s-api/{}/api/v1/namespaces/{}/pods", cluster, namespace);
        let resp = self.api_req(&path).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        let json: serde_json::Value = serde_json::from_str(&text).map_err(|_| {
            anyhow!(
                "Pod 列表接口返回非 JSON 数据 (HTTP {}, body: {})",
                status.as_u16(),
                &text[..text.len().min(500)]
            )
        })?;

        let pods: Vec<PodInfo> = json["items"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|pod| {
                let metadata = &pod["metadata"];
                let spec = &pod["spec"];
                let status = &pod["status"];
                let name = metadata["name"].as_str().unwrap_or("").to_string();
                let ip = status["podIP"].as_str().unwrap_or("").to_string();
                let phase = status["phase"].as_str().unwrap_or("Unknown").to_string();

                let containers: Vec<ContainerInfo> = spec["containers"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|c| {
                        let ports: Vec<ContainerPort> = c["ports"]
                            .as_array()
                            .unwrap_or(&vec![])
                            .iter()
                            .map(|p| ContainerPort {
                                name: p["name"].as_str().map(String::from),
                                container_port: p["containerPort"].as_u64().unwrap_or(0) as u16,
                                protocol: p["protocol"].as_str().unwrap_or("TCP").to_string(),
                            })
                            .collect();
                        ContainerInfo {
                            name: c["name"].as_str().unwrap_or("").to_string(),
                            ports,
                        }
                    })
                    .collect();

                PodInfo {
                    name,
                    ip,
                    status: phase,
                    containers,
                }
            })
            .collect();

        Ok(pods)
    }
}

fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect::<String>()
}

fn parse_login_message(msg: &str) -> LoginResult {
    if msg.contains("Login error") {
        let parts: Vec<&str> = msg.split(':').collect();
        if parts.len() >= 2 {
            match parts[1].trim() {
                "PASS" => LoginResult {
                    success: true,
                    mfa_required: None,
                    message: None,
                },
                "MFA_REQUIRED" => LoginResult {
                    success: false,
                    mfa_required: Some(true),
                    message: Some("需要双因子认证".into()),
                },
                "USER_NOT_FOUND" => LoginResult {
                    success: false,
                    mfa_required: None,
                    message: Some("用户名未找到".into()),
                },
                "WRONG_PASSWORD" => LoginResult {
                    success: false,
                    mfa_required: None,
                    message: Some("密码错误".into()),
                },
                "WRONG_PASSCODE" => LoginResult {
                    success: false,
                    mfa_required: None,
                    message: Some("验证码错误".into()),
                },
                _ => LoginResult {
                    success: false,
                    mfa_required: None,
                    message: Some(msg.to_string()),
                },
            }
        } else {
            LoginResult {
                success: false,
                mfa_required: None,
                message: Some(msg.to_string()),
            }
        }
    } else {
        LoginResult {
            success: false,
            mfa_required: None,
            message: Some(msg.to_string()),
        }
    }
}
