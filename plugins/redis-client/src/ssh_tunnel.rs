use crate::connection::{SshAuth, SshConfig};
use ssh2::Session;
use std::net::TcpStream;
use std::time::Duration;

pub struct SshTunnel {
    session: Session,
    local_port: u16,
    _listener: std::net::TcpListener,
}

impl SshTunnel {
    /// Establish SSH connection and create local port forwarding to target Redis
    /// Returns local_port for redis crate to connect to localhost:<port>
    pub fn connect(
        config: &SshConfig,
        remote_host: &str,
        remote_port: u16,
    ) -> Result<u16, String> {
        let addr = format!("{}:{}", config.host, config.port);
        let tcp = TcpStream::connect(&addr)
            .map_err(|e| format!("SSH TCP connect failed: {e}"))?;
        tcp.set_read_timeout(Some(Duration::from_secs(
            config.timeout_secs as u64,
        )))
        .ok();

        let mut session = Session::new()
            .map_err(|e| format!("SSH session creation failed: {e}"))?;
        session.set_tcp_stream(tcp);
        session
            .handshake()
            .map_err(|e| format!("SSH handshake failed: {e}"))?;

        match &config.auth {
            SshAuth::Password {
                password_obfuscated,
            } => {
                let pass =
                    crate::hex::deobfuscate(password_obfuscated).unwrap_or_default();
                session
                    .userauth_password(&config.username, &pass)
                    .map_err(|e| format!("SSH password auth failed: {e}"))?;
            }
            SshAuth::KeyPath {
                key_path,
                passphrase_obfuscated,
            } => {
                let passphrase = passphrase_obfuscated
                    .as_ref()
                    .and_then(|p| crate::hex::deobfuscate(p));
                session
                    .userauth_pubkey_file(
                        &config.username,
                        None,
                        std::path::Path::new(key_path),
                        passphrase.as_deref(),
                    )
                    .map_err(|e| format!("SSH key auth failed: {e}"))?;
            }
        }

        if !session.authenticated() {
            return Err("SSH authentication failed".into());
        }

        // Local port forwarding: localhost:0 → remote_host:remote_port
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("Local port bind failed: {e}"))?;
        let local_port = listener
            .local_addr()
            .map_err(|e| format!("Get local port failed: {e}"))?
            .port();

        // Forwarding thread
        let remote_host_owned = remote_host.to_string();
        let _thread = std::thread::spawn(move || {
            for stream in listener.incoming() {
                if let Ok(stream) = stream {
                    if let Ok(mut channel) = session.channel_direct_tcpip(
                        &remote_host_owned,
                        remote_port,
                        None,
                    ) {
                        let mut reader = stream.try_clone().unwrap();
                        let mut writer = stream;
                        std::io::copy(&mut reader, &mut channel).ok();
                        std::io::copy(&mut channel, &mut writer).ok();
                    }
                }
            }
        });

        Ok(local_port)
    }

    /// Verify SSH connectivity (handshake + auth only, no forwarding)
    pub fn test_connect(config: &SshConfig) -> Result<(), String> {
        let addr = format!("{}:{}", config.host, config.port);
        let tcp = TcpStream::connect(&addr)
            .map_err(|e| format!("SSH TCP connect failed: {e}"))?;
        tcp.set_read_timeout(Some(Duration::from_secs(
            config.timeout_secs as u64,
        )))
        .ok();

        let mut session = Session::new()
            .map_err(|e| format!("SSH session creation failed: {e}"))?;
        session.set_tcp_stream(tcp);
        session
            .handshake()
            .map_err(|e| format!("SSH handshake failed: {e}"))?;

        match &config.auth {
            SshAuth::Password {
                password_obfuscated,
            } => {
                let pass =
                    crate::hex::deobfuscate(password_obfuscated).unwrap_or_default();
                session
                    .userauth_password(&config.username, &pass)
                    .map_err(|e| format!("SSH password auth failed: {e}"))?;
            }
            SshAuth::KeyPath {
                key_path,
                passphrase_obfuscated,
            } => {
                let passphrase = passphrase_obfuscated
                    .as_ref()
                    .and_then(|p| crate::hex::deobfuscate(p));
                session
                    .userauth_pubkey_file(
                        &config.username,
                        None,
                        std::path::Path::new(key_path),
                        passphrase.as_deref(),
                    )
                    .map_err(|e| format!("SSH key auth failed: {e}"))?;
            }
        }

        if !session.authenticated() {
            return Err("SSH authentication failed".into());
        }
        Ok(())
    }
}
