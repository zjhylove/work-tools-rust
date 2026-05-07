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

use crate::models::{ForwardRule, RuleType};
use anyhow::{anyhow, Result};
use ssh2::Session;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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
        }
    }

    pub fn is_connected(&self) -> bool {
        self.session
            .as_ref()
            .map(|s| s.lock().unwrap().authenticated())
            .unwrap_or(false)
    }

    pub fn connect(&mut self, host: &str, port: u16, username: &str, password: &str) -> Result<()> {
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
        self.session = Some(Arc::new(Mutex::new(session)));
        Ok(())
    }

    pub fn disconnect(&mut self) {
        for flag in &self.stop_flags {
            *flag.lock().unwrap() = true;
        }
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
        self.stop_flags.clear();
        self.forwards.clear();
        self.session = None;
    }

    /// 添加端口转发规则，返回实际使用的本地端口
    pub fn add_forward(
        &mut self,
        local_host: &str,
        remote_host: &str,
        remote_port: u16,
        local_port: u16,
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

            loop {
                if *stop.lock().unwrap() {
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
                                if *stop.lock().unwrap() {
                                    return;
                                }
                                let s = session.lock().unwrap();
                                match s.channel_direct_tcpip(&rh, remote_port, None) {
                                    Ok(c) => {
                                        channel = Some(c);
                                        break;
                                    }
                                    Err(e) => {
                                        drop(s);
                                        let io: std::io::Error = e.into();
                                        if io.kind() == std::io::ErrorKind::WouldBlock {
                                            thread::sleep(Duration::from_millis(50));
                                            continue;
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
                    if *stop.lock().unwrap() {
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
                                let _lock = session.lock().unwrap();
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
                            let _lock = session.lock().unwrap();
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
            *entry.stop_flag.lock().unwrap() = true;
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
