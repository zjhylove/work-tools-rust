use crate::connection::{SshAuth, SshConfig};
use ssh2::Session;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct SshTunnel {
    handle: Option<JoinHandle<()>>,
    _session: Option<Arc<Mutex<Session>>>,
    local_port: u16,
    stop_flag: Arc<AtomicBool>,
}

impl SshTunnel {
    pub fn local_port(&self) -> u16 {
        self.local_port
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(session) = self._session.take() {
            if let Ok(s) = session.lock() {
                s.set_timeout(1);
                let _ = s.disconnect(None, "closing", None);
            }
        }
        if let Ok(stream) = TcpStream::connect(format!("127.0.0.1:{}", self.local_port)) {
            drop(stream);
        }
        if let Some(h) = self.handle.take() {
            h.join().ok();
        }
    }
}

fn create_authenticated_session(config: &SshConfig) -> Result<Session, String> {
    if config.host.trim().is_empty() {
        return Err("SSH 主机地址不能为空".to_string());
    }
    let addr = format!("{}:{}", config.host, config.port);
    let timeout = Duration::from_secs(config.timeout_secs as u64);
    let sock_addrs: Vec<_> = addr
        .to_socket_addrs()
        .map_err(|e| format!("SSH 地址解析失败: {e}"))?
        .collect();
    if sock_addrs.is_empty() {
        return Err("SSH 地址解析失败: 无可用地址".to_string());
    }
    let mut last_err = String::new();
    for sock_addr in &sock_addrs {
        let tcp = match TcpStream::connect_timeout(sock_addr, timeout) {
            Ok(tcp) => tcp,
            Err(e) => {
                last_err = format!("SSH TCP connect to {sock_addr} failed: {e}");
                continue;
            }
        };
        tcp.set_read_timeout(Some(timeout)).ok();
        tcp.set_write_timeout(Some(timeout)).ok();

        let mut session = match Session::new() {
            Ok(s) => s,
            Err(e) => {
                last_err = format!("SSH session creation failed: {e}");
                continue;
            }
        };
        session.set_tcp_stream(tcp);
        session.set_timeout(timeout.as_millis() as u32);
        if let Err(e) = session.handshake() {
            last_err = format!("SSH handshake failed: {e}");
            continue;
        }
        match &config.auth {
            SshAuth::Password { password_obfuscated } => {
                let pass = crate::hex::deobfuscate(password_obfuscated)
                    .unwrap_or_else(|| password_obfuscated.clone());
                if let Err(e) = session.userauth_password(&config.username, &pass) {
                    last_err = format!("SSH password auth failed: {e}");
                    continue;
                }
            }
            SshAuth::KeyPath { key_path, passphrase_obfuscated } => {
                let passphrase = passphrase_obfuscated.as_ref()
                    .and_then(|p| crate::hex::deobfuscate(p).or_else(|| Some(p.clone())));
                if let Err(e) = session.userauth_pubkey_file(
                    &config.username,
                    None,
                    std::path::Path::new(key_path),
                    passphrase.as_deref(),
                ) {
                    last_err = format!("SSH key auth failed: {e}");
                    continue;
                }
            }
        }
        if session.authenticated() {
            session.set_timeout(0);
            return Ok(session);
        }
        last_err = "SSH authentication failed".to_string();
    }
    Err(last_err)
}

/// Single-threaded forwarding loop using non-blocking I/O to avoid
/// the deadlock that occurs when two threads compete for a Mutex on
/// the SSH channel's read/write operations.
fn forward_loop(
    session: &Session,
    mut tcp: TcpStream,
    channel: &mut ssh2::Channel,
    stop: &AtomicBool,
) {
    session.set_blocking(false);
    tcp.set_nonblocking(true).ok();

    let mut tcp_buf = [0u8; 8192];
    let mut ch_buf = [0u8; 8192];

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        let mut activity = false;

        // remote -> local
        match channel.read(&mut ch_buf) {
            Ok(0) => break,
            Ok(n) => {
                tcp.set_nonblocking(false).ok();
                if tcp.write_all(&ch_buf[..n]).is_err() {
                    break;
                }
                tcp.set_nonblocking(true).ok();
                activity = true;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }

        // local -> remote
        match tcp.read(&mut tcp_buf) {
            Ok(0) => break,
            Ok(n) => {
                if channel.write_all(&tcp_buf[..n]).is_err() {
                    break;
                }
                channel.flush().ok();
                activity = true;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }

        if !activity {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    session.set_blocking(true);
    tcp.set_nonblocking(false).ok();
}

impl SshTunnel {
    pub fn connect(
        config: &SshConfig,
        remote_host: &str,
        remote_port: u16,
    ) -> Result<Self, String> {
        let session = create_authenticated_session(config)?;
        let session = Arc::new(Mutex::new(session));

        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("Local port bind failed: {e}"))?;
        let local_port = listener.local_addr()
            .map_err(|e| format!("Get local port failed: {e}"))?
            .port();

        listener.set_nonblocking(true).ok();

        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_for_thread = Arc::clone(&stop_flag);
        let session_for_thread = Arc::clone(&session);
        let rh = remote_host.to_string();

        let handle = thread::Builder::new()
            .name("redis-ssh-fwd".into())
            .spawn(move || {
                loop {
                    if stop_for_thread.load(Ordering::Relaxed) {
                        break;
                    }

                    let stream = match listener.accept() {
                        Ok((s, _)) => s,
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(50));
                            continue;
                        }
                        Err(_) => break,
                    };

                    // Hold the session lock to open channel AND run forwarding.
                    // This is safe because forwarding is single-threaded and non-blocking.
                    let sess = session_for_thread.lock().unwrap();
                    let mut channel = match sess.channel_direct_tcpip(&rh, remote_port, None) {
                        Ok(ch) => ch,
                        Err(e) => {
                            tracing::warn!(?e, "ssh channel_direct_tcpip failed");
                            continue;
                        }
                    };

                    forward_loop(&sess, stream, &mut channel, &stop_for_thread);
                    drop(channel);
                }
            })
            .map_err(|e| format!("Failed to spawn forwarding thread: {e}"))?;

        Ok(SshTunnel {
            handle: Some(handle),
            _session: Some(session),
            local_port,
            stop_flag,
        })
    }
}
