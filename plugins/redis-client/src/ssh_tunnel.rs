use crate::connection::{SshAuth, SshConfig};
use ssh2::Session;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct SshTunnel {
    handle: Option<JoinHandle<()>>,
    session: Option<Session>,
    _listener: TcpListener,
    local_port: u16,
}

impl SshTunnel {
    pub fn local_port(&self) -> u16 {
        self.local_port
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        // Drop session first to terminate active channels
        self.session.take();
        // Join the forwarding thread
        if let Some(h) = self.handle.take() {
            h.join().ok();
        }
    }
}

fn create_authenticated_session(config: &SshConfig) -> Result<Session, String> {
    let addr = format!("{}:{}", config.host, config.port);
    let tcp = TcpStream::connect(&addr)
        .map_err(|e| format!("SSH TCP connect failed: {e}"))?;
    tcp.set_read_timeout(Some(Duration::from_secs(config.timeout_secs as u64)))
        .ok();

    let mut session = Session::new()
        .map_err(|e| format!("SSH session creation failed: {e}"))?;
    session.set_tcp_stream(tcp);
    session.handshake()
        .map_err(|e| format!("SSH handshake failed: {e}"))?;

    match &config.auth {
        SshAuth::Password { password_obfuscated } => {
            let pass = crate::hex::deobfuscate(password_obfuscated)
                .ok_or_else(|| "Stored password data is corrupted".to_string())?;
            session.userauth_password(&config.username, &pass)
                .map_err(|e| format!("SSH password auth failed: {e}"))?;
        }
        SshAuth::KeyPath { key_path, passphrase_obfuscated } => {
            let passphrase = passphrase_obfuscated.as_ref()
                .and_then(|p| crate::hex::deobfuscate(p));
            session.userauth_pubkey_file(
                &config.username,
                None,
                std::path::Path::new(key_path),
                passphrase.as_deref(),
            ).map_err(|e| format!("SSH key auth failed: {e}"))?;
        }
    }

    if !session.authenticated() {
        return Err("SSH authentication failed".into());
    }
    Ok(session)
}

impl SshTunnel {
    pub fn connect(
        config: &SshConfig,
        remote_host: &str,
        remote_port: u16,
    ) -> Result<Self, String> {
        let session = create_authenticated_session(config)?;

        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("Local port bind failed: {e}"))?;
        let local_port = listener.local_addr()
            .map_err(|e| format!("Get local port failed: {e}"))?
            .port();

        let rh = remote_host.to_string();

        let handle = thread::Builder::new()
            .name("redis-ssh-fwd".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    let mut stream = match stream {
                        Ok(s) => s,
                        Err(_) => break,
                    };

                    let mut channel = match session.channel_direct_tcpip(
                        &rh,
                        remote_port,
                        None,
                    ) {
                        Ok(ch) => ch,
                        Err(e) => {
                            tracing::warn!(?e, "ssh channel_direct_tcpip failed");
                            continue;
                        }
                    };

                    // Bidirectional forwarding: two threads with shared channel
                    let ch = Arc::new(Mutex::new(channel));
                    let mut reader = match stream.try_clone() {
                        Ok(r) => r,
                        Err(e) => {
                            tracing::warn!(?e, "stream try_clone failed");
                            continue;
                        }
                    };
                    let mut writer = stream;

                    let ch_reader = Arc::clone(&ch);

                    // local -> remote
                    let t1 = thread::Builder::new()
                        .name("ssh-fwd-local-to-remote".into())
                        .spawn(move || {
                            let mut buf = [0u8; 8192];
                            loop {
                                let n = match reader.read(&mut buf) {
                                    Ok(0) => break,
                                    Ok(n) => n,
                                    Err(e) => {
                                        tracing::warn!(?e, "read from local stream failed");
                                        break;
                                    }
                                };
                                let mut ch = match ch_reader.lock() {
                                    Ok(c) => c,
                                    Err(_) => break,
                                };
                                if ch.write_all(&buf[..n]).is_err() {
                                    break;
                                }
                            }
                        })
                        .unwrap();

                    // remote -> local
                    let t2 = thread::Builder::new()
                        .name("ssh-fwd-remote-to-local".into())
                        .spawn(move || {
                            let mut buf = [0u8; 8192];
                            loop {
                                let n = {
                                    let mut ch = match ch.lock() {
                                        Ok(c) => c,
                                        Err(_) => break,
                                    };
                                    match ch.read(&mut buf) {
                                        Ok(0) => break,
                                        Ok(n) => n,
                                        Err(e) => {
                                            tracing::warn!(?e, "read from ssh channel failed");
                                            break;
                                        }
                                    }
                                };
                                if writer.write_all(&buf[..n]).is_err() {
                                    break;
                                }
                            }
                        })
                        .unwrap();

                    t1.join().ok();
                    t2.join().ok();
                }
            })
            .map_err(|e| format!("Failed to spawn forwarding thread: {e}"))?;

        Ok(SshTunnel {
            handle: Some(handle),
            session: Some(session),
            _listener: listener,
            local_port,
        })
    }

    pub fn test_connect(config: &SshConfig) -> Result<(), String> {
        let _session = create_authenticated_session(config)?;
        Ok(())
    }
}
