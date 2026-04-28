use anyhow::{Result, anyhow};
use reqwest::{Client, cookie::Jar};
use std::sync::Arc;
use crate::models::*;

pub struct KuboardClient {
    client: Client,
    base_url: String,
    logged_in: bool,
    username: String,
    password: String,
}

impl KuboardClient {
    pub fn new(base_url: &str) -> Self {
        let cookie_jar = Arc::new(Jar::default());
        let client = Client::builder()
            .cookie_provider(cookie_jar)
            .danger_accept_invalid_certs(false)
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            logged_in: false,
            username: String::new(),
            password: String::new(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Step 1: 从 redirect 获取 SSO req_id
    async fn fetch_req_id(&self) -> Result<String> {
        let resp = self.client
            .get(&self.url("/kuboard/cluster"))
            .send()
            .await?;
        let final_url = resp.url().to_string();
        if let Some(pos) = final_url.find("req=") {
            return Ok(final_url[pos + 4..].to_string());
        }
        let resp = self.client
            .get(&self.url("/login?state=%2Fkuboard%2Fcluster"))
            .send()
            .await?;
        let final_url = resp.url().to_string();
        if let Some(pos) = final_url.find("req=") {
            return Ok(final_url[pos + 4..].to_string());
        }
        Err(anyhow!("无法获取 SSO req_id，请检查 Kuboard 地址"))
    }

    /// Step 2: POST 登录到 SSO
    pub async fn login(&mut self, username: &str, password: &str) -> Result<LoginResult> {
        self.username = username.to_string();
        self.password = password.to_string();

        let req_id = self.fetch_req_id().await?;

        let pwd_json = format!("{{\"password\":\"{}\"}}", password);
        let body = format!("login={}&password={}",
            urlencoding(username), urlencoding(&pwd_json));

        let resp = self.client
            .post(&self.url(&format!("/sso/auth/default?req={}", req_id)))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?;

        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(msg) = json["message"].as_str() {
                return Ok(parse_login_message(msg));
            }
        }

        if status == 302 || status == 303 || status == 200 {
            self.logged_in = true;
            return Ok(LoginResult { success: true, mfa_required: None, message: None });
        }

        Err(anyhow!("登录失败: HTTP {} - {}",
            status, text.chars().take(200).collect::<String>()))
    }

    /// Step 3: MFA 验证
    pub async fn mfa_verify(&mut self, passcode: &str) -> Result<()> {
        let resp = self.client
            .post(&self.url("/login/password"))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "username": self.username,
                "password": self.password,
                "passcode": passcode,
            }))
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
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

    pub fn is_logged_in(&self) -> bool { self.logged_in }

    pub fn username(&self) -> &str { &self.username }

    /// 获取集群列表
    pub async fn list_clusters(&self) -> Result<Vec<String>> {
        let resp = self.client
            .get(&self.url("/kuboard/api/clusters"))
            .send()
            .await?;
        let json: serde_json::Value = resp.json().await?;
        let clusters: Vec<String> = json.as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|c| c["name"].as_str().map(String::from))
            .collect();
        Ok(clusters)
    }

    /// 获取命名空间列表
    pub async fn list_namespaces(&self, cluster: &str) -> Result<Vec<String>> {
        let path = format!("/k8s-api/{}/api/v1/namespaces", cluster);
        let resp = self.client.get(&self.url(&path)).send().await?;
        let json: serde_json::Value = resp.json().await?;
        let namespaces: Vec<String> = json["items"].as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|ns| ns["metadata"]["name"].as_str().map(String::from))
            .collect();
        Ok(namespaces)
    }

    /// 获取 Pod 列表
    pub async fn list_pods(&self, cluster: &str, namespace: &str) -> Result<Vec<PodInfo>> {
        let path = format!("/k8s-api/{}/api/v1/namespaces/{}/pods", cluster, namespace);
        let resp = self.client.get(&self.url(&path)).send().await?;
        let json: serde_json::Value = resp.json().await?;

        let pods: Vec<PodInfo> = json["items"].as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|pod| {
                let metadata = &pod["metadata"];
                let spec = &pod["spec"];
                let status = &pod["status"];
                let name = metadata["name"].as_str().unwrap_or("").to_string();
                let ip = status["podIP"].as_str().unwrap_or("").to_string();
                let phase = status["phase"].as_str().unwrap_or("Unknown").to_string();

                let containers: Vec<ContainerInfo> = spec["containers"].as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|c| {
                        let ports: Vec<ContainerPort> = c["ports"].as_array()
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

                PodInfo { name, ip, status: phase, containers }
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
                "PASS" => LoginResult { success: true, mfa_required: None, message: None },
                "MFA_REQUIRED" => LoginResult { success: false, mfa_required: Some(true), message: Some("需要双因子认证".into()) },
                "USER_NOT_FOUND" => LoginResult { success: false, mfa_required: None, message: Some("用户名未找到".into()) },
                "WRONG_PASSWORD" => LoginResult { success: false, mfa_required: None, message: Some("密码错误".into()) },
                "WRONG_PASSCODE" => LoginResult { success: false, mfa_required: None, message: Some("验证码错误".into()) },
                _ => LoginResult { success: false, mfa_required: None, message: Some(msg.to_string()) },
            }
        } else {
            LoginResult { success: false, mfa_required: None, message: Some(msg.to_string()) }
        }
    } else {
        LoginResult { success: false, mfa_required: None, message: Some(msg.to_string()) }
    }
}
