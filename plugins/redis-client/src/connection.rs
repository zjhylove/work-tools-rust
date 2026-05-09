use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: SshAuth,
    pub timeout_secs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SshAuth {
    #[serde(rename = "password")]
    Password { password_obfuscated: String },
    #[serde(rename = "key")]
    KeyPath { key_path: String, passphrase_obfuscated: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub seed_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub host: String,
    pub port: u16,
    pub db: i64,
    #[serde(default)]
    pub password_obfuscated: String,
    #[serde(default)]
    pub ssh: Option<SshConfig>,
    #[serde(default)]
    pub cluster: Option<ClusterConfig>,
}

/// Connection mode (runtime decision for connection path)
pub enum ConnectionMode {
    Direct {
        host: String,
        port: u16,
        db: i64,
        password: Option<String>,
    },
    SshTunnel {
        ssh: SshConfig,
        remote_host: String,
        remote_port: u16,
        db: i64,
        password: Option<String>,
    },
    Cluster {
        seed_nodes: Vec<String>,
        password: Option<String>,
    },
}

impl ConnectionConfig {
    /// Build a ConnectionMode for establishing Redis connection
    pub fn to_connection_mode(&self, password: Option<String>) -> ConnectionMode {
        match (&self.ssh, &self.cluster) {
            (Some(ssh), _) => ConnectionMode::SshTunnel {
                ssh: ssh.clone(),
                remote_host: self.host.clone(),
                remote_port: self.port,
                db: self.db,
                password,
            },
            (_, Some(cluster)) => ConnectionMode::Cluster {
                seed_nodes: cluster.seed_nodes.clone(),
                password,
            },
            _ => ConnectionMode::Direct {
                host: self.host.clone(),
                port: self.port,
                db: self.db,
                password,
            },
        }
    }
}
