//! # SSH 端口转发服务
//!
//! 基于 ssh2 crate 实现 SSH 隧道和本地端口转发。
//!
//! ## 工作原理
//! ```
//! 用户程序 → localhost:10000 (TCP listener)
//!   → std::io::copy(本地socket, SSH channel)
//!   → SSH 隧道 (加密传输)
//!   → 远程主机:端口 (channel_direct_tcpip)
//! ```
//!
//! 每个转发规则一个独立线程，线程内通过非阻塞 I/O 复用多个并发连接：
//! 1. 非阻塞 accept 新连接 → 创建 SSH channel → 加入活跃连接列表
//! 2. 轮询所有活跃连接：local→remote、remote→local 双向转发
//! 3. 检测到 EOF/错误 → 从列表移除该连接
//! 4. 无工作时短暂休眠避免忙循环
//!
//! ## 线程安全
//! 所有 channel 操作都在 session 锁保护下执行。同一 forward 的多个连接
//! 在同一线程内串行处理无需额外同步；不同 forward 之间通过 session 锁
//! 互斥，锁持有时间极短（非阻塞操作立刻返回），无锁竞争问题。

use crate::models::{ForwardRule, ReconnectInfo, RuleType, SshConnectionState};
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use ssh2::Session;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// 保存的 SSH 连接参数，供重连使用
struct ConnectParams {
    host: String,
    port: u16,
    username: String,
    password: String,
}

/// 重连状态
struct ReconnectState {
    retry_count: u32,
    max_retries: u32,
    next_retry_at: std::time::Instant,
}

/// 活跃的转发连接
struct ActiveConn {
    local_read: TcpStream,
    local_write: TcpStream,
    channel: ssh2::Channel,
}

pub struct SshService {
    session: Option<Arc<Mutex<Session>>>,
    forwards: Vec<ForwardEntry>,
    next_port: u16,
    threads: Vec<thread::JoinHandle<()>>,
    stop_flags: Vec<Arc<Mutex<bool>>>,
    connect_params: Option<ConnectParams>,
    heartbeat_stop: Arc<Mutex<bool>>,
    heartbeat_thread: Option<thread::JoinHandle<()>>,
    reconnect_stop: Arc<Mutex<bool>>,
    reconnect_state: Arc<Mutex<Option<ReconnectState>>>,
    reconnect_thread: Option<thread::JoinHandle<()>>,
    manual_disconnect: bool,
    reconnect_exhausted: bool,
}

impl Default for SshService {
    fn default() -> Self {
        Self::new()
    }
}

struct ForwardEntry {
    rule: ForwardRule,
    stop_flag: Arc<Mutex<bool>>,
}

impl SshService {
    pub fn new() -> Self {
        Self {
            session: None,
            forwards: vec![],
            next_port: 10000,
            threads: vec![],
            stop_flags: vec![],
            connect_params: None,
            heartbeat_stop: Arc::new(Mutex::new(false)),
            heartbeat_thread: None,
            reconnect_stop: Arc::new(Mutex::new(false)),
            reconnect_state: Arc::new(Mutex::new(None)),
            reconnect_thread: None,
            manual_disconnect: false,
            reconnect_exhausted: false,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.session
            .as_ref()
            .map(|s| s.lock().authenticated())
            .unwrap_or(false)
    }

    pub fn has_connect_params(&self) -> bool {
        self.connect_params.is_some()
    }

    pub fn is_reconnecting(&self) -> bool {
        self.reconnect_state.lock().is_some()
    }

    pub fn is_reconnect_exhausted(&self) -> bool {
        self.reconnect_exhausted
    }

    /// 检查是否有转发线程异常退出
    pub fn any_forward_thread_exited(&self) -> bool {
        self.threads.iter().any(|h| h.is_finished())
    }

    pub fn connection_state(&self) -> SshConnectionState {
        if self.reconnect_state.lock().is_some() {
            SshConnectionState::Reconnecting
        } else if self.is_connected() {
            SshConnectionState::Connected
        } else {
            SshConnectionState::Disconnected
        }
    }

    pub fn manual_disconnect(&self) -> bool {
        self.manual_disconnect
    }

    pub fn set_manual_disconnect(&mut self, value: bool) {
        self.manual_disconnect = value;
    }

    pub fn get_reconnect_info(&self) -> Option<ReconnectInfo> {
        self.reconnect_state.lock().as_ref().map(|rs| {
            let duration_until_retry = rs
                .next_retry_at
                .checked_duration_since(std::time::Instant::now())
                .unwrap_or(std::time::Duration::from_secs(0));
            let next_retry_time = std::time::SystemTime::now()
                + std::time::Duration::from_secs(duration_until_retry.as_secs());
            ReconnectInfo {
                retry_count: rs.retry_count,
                max_retries: rs.max_retries,
                next_retry_at: next_retry_time
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or(std::time::Duration::from_secs(0))
                    .as_secs(),
            }
        })
    }

    pub fn connect(&mut self, host: &str, port: u16, username: &str, password: &str) -> Result<()> {
        let session = Self::create_session(host, port, username, password)?;

        self.connect_params = Some(ConnectParams {
            host: host.to_string(),
            port,
            username: username.to_string(),
            password: password.to_string(),
        });

        self.session = Some(Arc::new(Mutex::new(session)));
        self.reconnect_exhausted = false;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.stop_reconnect();
        self.stop_heartbeat();

        for flag in &self.stop_flags {
            *flag.lock() = true;
        }
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
        self.stop_flags.clear();
        self.forwards.clear();
        self.session = None;
    }

    /// 停止所有转发线程并释放 TCP 端口，但保持 SSH 会话不断开。
    /// 用于自动重连成功后清理旧会话残留的转发线程。
    pub fn stop_forwards(&mut self) {
        let count = self.threads.len();
        for flag in &self.stop_flags {
            *flag.lock() = true;
        }
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
        self.stop_flags.clear();
        self.forwards.clear();
        if count > 0 {
            tracing::info!("已停止 {} 个旧转发线程", count);
        }
    }

    /// 添加端口转发规则，返回实际使用的本地端口
    pub fn add_forward(
        &mut self,
        local_host: &str,
        remote_host: &str,
        remote_port: u16,
        local_port: u16,
        ssh_connection_id: &str,
    ) -> Result<u16> {
        let session = self.session.clone().ok_or_else(|| anyhow!("SSH 未连接"))?;

        let (listener, local_port) = if local_port > 0 {
            let addr = format!("{}:{}", local_host, local_port);
            let listener = TcpListener::bind(&addr)
                .map_err(|e| anyhow!("端口 {} 绑定失败: {}", local_port, e))?;
            (listener, local_port)
        } else {
            self.bind_auto_port(local_host)?
        };
        listener.set_nonblocking(true).ok();

        let rh = remote_host.to_string();
        let rh_for_rule = remote_host.to_string();
        let stop_flag = Arc::new(Mutex::new(false));
        let stop = stop_flag.clone();
        let stop_for_entry = stop_flag.clone();

        let handle = thread::spawn(move || {
            let mut connections: Vec<ActiveConn> = Vec::new();
            let mut buf = [0u8; 8192];
            let mut channel_fail_count = 0u32;

            loop {
                if *stop.lock() {
                    return;
                }
                let mut did_work = false;

                // 1. 尝试 accept 新连接
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream.set_nonblocking(true).ok();
                        if let Ok(local_read) = stream.try_clone() {
                            let local_write = stream;
                            let mut channel = None;
                            loop {
                                if *stop.lock() {
                                    return;
                                }
                                let s = session.lock();
                                match s.channel_direct_tcpip(&rh, remote_port, None) {
                                    Ok(c) => {
                                        channel = Some(c);
                                        channel_fail_count = 0;
                                        break;
                                    }
                                    Err(e) => {
                                        drop(s);
                                        let io: std::io::Error = e.into();
                                        if io.kind() == std::io::ErrorKind::WouldBlock {
                                            thread::sleep(Duration::from_millis(50));
                                            continue;
                                        }
                                        // 非 WouldBlock 错误：SSH 会话可能已断开
                                        channel_fail_count += 1;
                                        if channel_fail_count >= 10 {
                                            tracing::warn!("转发线程连续 {} 次创建 channel 失败，SSH 会话可能已断开", channel_fail_count);
                                            return;
                                        }
                                        break;
                                    }
                                }
                            }
                            if let Some(ch) = channel {
                                connections.push(ActiveConn {
                                    local_read,
                                    local_write,
                                    channel: ch,
                                });
                                did_work = true;
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(_) => return,
                }

                // 2. 服务所有活跃连接
                let mut i = 0;
                while i < connections.len() {
                    if *stop.lock() {
                        return;
                    }

                    let conn = &mut connections[i];
                    let mut dead = false;

                    // 本地 → 远程
                    match conn.local_read.read(&mut buf) {
                        Ok(0) => dead = true,
                        Ok(n) => {
                            did_work = true;
                            let write_ok = {
                                let _lock = session.lock();
                                conn.channel.write_all(&buf[..n]).is_ok()
                            };
                            if !write_ok {
                                dead = true;
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(_) => dead = true,
                    }

                    // 远程 → 本地
                    if !dead {
                        let channel_result = {
                            let _lock = session.lock();
                            conn.channel.read(&mut buf)
                        };
                        match channel_result {
                            Ok(0) => dead = true,
                            Ok(n) => {
                                did_work = true;
                                let _ = conn.local_write.write_all(&buf[..n]);
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                            Err(_) => dead = true,
                        }
                    }

                    if dead {
                        connections.swap_remove(i);
                    } else {
                        i += 1;
                    }
                }

                if !did_work {
                    thread::sleep(Duration::from_millis(10));
                }
            }
        });

        let rule = ForwardRule {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!("forward-{}", local_port),
            local_host: local_host.to_string(),
            local_port,
            remote_host: rh_for_rule,
            remote_port,
            rule_type: RuleType::Manual,
            cluster: None,
            namespace: None,
            pod_name: None,
            container_name: None,
            ssh_connection_id: ssh_connection_id.to_string(),
        };

        self.threads.push(handle);
        self.stop_flags.push(stop_flag);
        self.forwards.push(ForwardEntry {
            rule,
            stop_flag: stop_for_entry,
        });

        Ok(local_port)
    }

    /// 移除端口转发
    pub fn remove_forward(&mut self, local_port: u16) -> Result<()> {
        if let Some(pos) = self
            .forwards
            .iter()
            .position(|f| f.rule.local_port == local_port)
        {
            let entry = self.forwards.remove(pos);
            *entry.stop_flag.lock() = true;
            self.stop_flags.remove(pos);
            let handle = self.threads.remove(pos);
            let _ = handle.join();
        }
        Ok(())
    }

    pub fn list_forwards(&self) -> Vec<ForwardRule> {
        self.forwards.iter().map(|f| f.rule.clone()).collect()
    }

    pub fn forward_count(&self) -> usize {
        self.forwards.len()
    }

    /// 启动心跳检测线程
    ///
    /// 通过实际创建 SSH channel 来探测连接存活，而非仅检查 `authenticated()`。
    /// `authenticated()` 返回的是初始认证时的缓存状态，TCP 连接断开后仍返回 true，
    /// 无法检测到防火墙/NAT 空闲超时导致的静默断连。
    pub fn start_heartbeat(&mut self) {
        self.stop_heartbeat();

        *self.heartbeat_stop.lock() = false;
        let stop = self.heartbeat_stop.clone();
        let session_check = self.session.clone();

        let handle = thread::spawn(move || {
            let mut fail_count = 0u32;
            loop {
                if *stop.lock() {
                    return;
                }
                let sleep_until = std::time::Instant::now() + Duration::from_secs(15);
                while std::time::Instant::now() < sleep_until {
                    if *stop.lock() {
                        return;
                    }
                    thread::sleep(Duration::from_secs(1));
                }

                let alive = probe_connection(&session_check);

                if alive {
                    fail_count = 0;
                } else {
                    fail_count += 1;
                    tracing::warn!("SSH 心跳探测失败 ({}/2)", fail_count);
                    if fail_count >= 2 {
                        tracing::warn!("SSH 心跳探测连续失败 {} 次，判定连接已断开", fail_count);
                        return;
                    }
                }
            }
        });

        self.heartbeat_thread = Some(handle);
    }

    /// 停止心跳检测线程
    pub fn stop_heartbeat(&mut self) {
        *self.heartbeat_stop.lock() = true;
        if let Some(handle) = self.heartbeat_thread.take() {
            let _ = handle.join();
        }
    }

    /// 检查心跳线程是否已退出（退出表示检测到断连）
    pub fn heartbeat_exited(&self) -> bool {
        self.heartbeat_thread
            .as_ref()
            .map(|h| h.is_finished())
            .unwrap_or(true)
    }

    /// 启动自动重连
    pub fn start_reconnect(&mut self) {
        self.stop_reconnect();
        self.reconnect_exhausted = false;

        let params = match &self.connect_params {
            Some(p) => ConnectParams {
                host: p.host.clone(),
                port: p.port,
                username: p.username.clone(),
                password: p.password.clone(),
            },
            None => return,
        };

        *self.reconnect_state.lock() = Some(ReconnectState {
            retry_count: 0,
            max_retries: 10,
            next_retry_at: std::time::Instant::now(),
        });

        let stop = self.reconnect_stop.clone();
        let session_slot = self.session.clone();
        let state = self.reconnect_state.clone();

        let host = params.host.clone();
        let port = params.port;
        let username = params.username.clone();
        let password = params.password.clone();

        let handle = thread::spawn(move || {
            let mut delay = Duration::from_secs(2);
            let max_delay = Duration::from_secs(60);

            for attempt in 1..=10u32 {
                if *stop.lock() {
                    tracing::info!("SSH 重连已取消");
                    return;
                }

                // 更新重试计数
                {
                    let mut s = state.lock();
                    if let Some(ref mut rs) = *s {
                        rs.retry_count = attempt;
                        rs.next_retry_at = std::time::Instant::now() + delay;
                    }
                }

                tracing::info!(
                    "SSH 重连尝试 {}/10，{} 秒后执行...",
                    attempt,
                    delay.as_secs()
                );
                let sleep_until = std::time::Instant::now() + delay;
                while std::time::Instant::now() < sleep_until {
                    if *stop.lock() {
                        tracing::info!("SSH 重连已取消");
                        return;
                    }
                    thread::sleep(Duration::from_secs(1));
                }

                match Self::create_session(&host, port, &username, &password) {
                    Ok(session) => {
                        tracing::info!("SSH 重连成功（第 {} 次尝试）", attempt);
                        if let Some(slot) = session_slot.as_ref() {
                            *slot.lock() = session;
                        }
                        return;
                    }
                    Err(e) => {
                        tracing::warn!("SSH 重连失败（第 {} 次）: {}", attempt, e);
                    }
                }

                delay = std::cmp::min(delay * 2, max_delay);
            }

            tracing::error!("SSH 重连失败，已耗尽 10 次重试");
        });

        self.reconnect_thread = Some(handle);
    }

    /// 停止重连
    pub fn stop_reconnect(&mut self) {
        *self.reconnect_stop.lock() = true;
        if let Some(handle) = self.reconnect_thread.take() {
            let _ = handle.join();
        }
        *self.reconnect_stop.lock() = false;
        *self.reconnect_state.lock() = None;
    }

    fn create_session(host: &str, port: u16, username: &str, password: &str) -> Result<Session> {
        let addr = format!("{}:{}", host, port);
        let tcp = TcpStream::connect(&addr)?;
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;
        session.userauth_password(username, password)?;
        if !session.authenticated() {
            return Err(anyhow!("SSH 认证失败"));
        }
        session.set_blocking(false);
        Ok(session)
    }

    /// 检查重连线程是否完成
    pub fn check_reconnect_result(&mut self) -> Option<bool> {
        let handle = self.reconnect_thread.as_ref()?;
        if !handle.is_finished() {
            return None;
        }
        let handle = self.reconnect_thread.take().unwrap();
        let _ = handle.join();
        let connected = self.is_connected();
        *self.reconnect_state.lock() = None;
        if !connected {
            self.reconnect_exhausted = true;
        }
        Some(connected)
    }

    fn bind_auto_port(&mut self, local_host: &str) -> Result<(TcpListener, u16)> {
        let used_ports: Vec<u16> = self.forwards.iter().map(|f| f.rule.local_port).collect();
        loop {
            let port = self.next_port;
            self.next_port += 1;
            if self.next_port > 60000 {
                self.next_port = 10000;
            }
            if used_ports.contains(&port) {
                continue;
            }
            let addr = format!("{}:{}", local_host, port);
            if let Ok(listener) = TcpListener::bind(&addr) {
                return Ok((listener, port));
            }
        }
    }
}

/// 通过实际创建 SSH channel 来探测连接是否存活
///
/// 仅检查 `authenticated()` 不可靠——它返回初始认证的缓存状态，
/// TCP 连接被防火墙/NAT 静默丢弃后仍返回 true。
/// 此函数短暂切换到阻塞模式尝试创建 channel，能真正检测 TCP 层断连。
fn probe_connection(session: &Option<Arc<Mutex<Session>>>) -> bool {
    session
        .as_ref()
        .map(|s| {
            let session = s.lock();
            session.set_blocking(true);
            session.set_timeout(5000);
            let result = session.channel_session();
            session.set_blocking(false);
            session.set_timeout(0);
            result.is_ok()
        })
        .unwrap_or(false)
}
