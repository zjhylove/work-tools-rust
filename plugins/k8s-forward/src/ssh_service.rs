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
//! 每个转发规则启动一个独立的线程：
//! 1. 绑定本地端口（TCP Listener）
//! 2. 接受连接后，创建 SSH direct-tcpip channel
//! 3. 双向转发数据（本地 ↔ SSH Channel）
//!
//! ## Rust 知识点
//! - `thread::spawn`: 创建操作系统线程
//! - `Arc<Mutex<T>>`: 多线程共享数据
//! - `TcpListener`: 监听 TCP 连接
//! - `Session::channel_direct_tcpip`: 创建 SSH 直接转发通道
//! - `JoinHandle`: 线程句柄，用于等待线程结束
//! - `set_nonblocking`: 设置非阻塞模式（避免读/写阻塞整个线程）

use crate::models::{ForwardRule, RuleType};
use anyhow::{anyhow, Result};
use ssh2::Session;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// SSH 转发服务
///
/// ## 字段说明
/// - `session`: SSH 会话（Arc<Mutex<>> 允许多线程共享）
/// - `forwards`: 活跃的转发条目列表
/// - `next_port`: 自动分配端口的起始值（10000-60000）
/// - `threads`: 转发线程的 JoinHandle（用于 join 等待）
/// - `stop_flags`: 每个转发线程的停止信号
pub struct SshService {
    session: Option<Arc<Mutex<Session>>>,
    forwards: Vec<ForwardEntry>,
    next_port: u16,
    threads: Vec<thread::JoinHandle<()>>,
    stop_flags: Vec<Arc<Mutex<bool>>>,
}

/// 转发条目（内部追踪结构）
struct ForwardEntry {
    rule: ForwardRule,
    stop_flag: Arc<Mutex<bool>>, // 设置 true 时转发线程退出
}

impl SshService {
    pub fn new() -> Self {
        Self {
            session: None,
            forwards: vec![],
            next_port: 10000, // 自动端口分配从 10000 开始
            threads: vec![],
            stop_flags: vec![],
        }
    }

    /// 检查 SSH 是否已连接且已认证
    pub fn is_connected(&self) -> bool {
        self.session
            .as_ref()
            .map(|s| s.lock().unwrap().authenticated())
            .unwrap_or(false)
    }

    /// 建立 SSH 连接
    ///
    /// ## 连接步骤
    /// 1. TCP 连接到 SSH 服务器
    /// 2. SSH 握手（密钥交换）
    /// 3. 用户名/密码认证
    /// 4. 设置 non-blocking 模式（允许多个转发线程并发 IO）
    pub fn connect(&mut self, host: &str, port: u16, username: &str, password: &str) -> Result<()> {
        let addr = format!("{}:{}", host, port);
        let tcp = TcpStream::connect(&addr)?;
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?; // SSH 握手
        session.userauth_password(username, password)?; // 密码认证
        if !session.authenticated() {
            return Err(anyhow!("SSH 认证失败"));
        }
        session.set_blocking(false); // 全局 non-blocking，转发线程通过 EAGAIN 重试处理 channel 创建
        // 用 Arc<Mutex<>> 包装，允许多线程访问
        self.session = Some(Arc::new(Mutex::new(session)));
        Ok(())
    }

    /// 断开 SSH 连接并停止所有转发线程
    ///
    /// 清理顺序：
    /// 1. 设置所有转发线程的停止标志
    /// 2. join 等待所有线程退出（drain 取出并消耗）
    /// 3. 清除状态
    pub fn disconnect(&mut self) {
        // 通知所有转发线程停止
        for flag in &self.stop_flags {
            *flag.lock().unwrap() = true;
        }
        // join 等待所有线程退出
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
        self.stop_flags.clear();
        self.forwards.clear();
        self.session = None; // 释放 SSH 会话
    }

    /// 添加端口转发规则
    ///
    /// ## 参数
    /// - `local_port = 0`: 自动分配端口
    /// - `local_port > 0`: 使用指定端口
    ///
    /// ## 返回值
    /// 实际使用的本地端口号
    pub fn add_forward(
        &mut self,
        local_host: &str,
        remote_host: &str,
        remote_port: u16,
        local_port: u16,
    ) -> Result<u16> {
        let session = self.session.clone().ok_or_else(|| anyhow!("SSH 未连接"))?;

        // 自动分配或使用指定端口
        let local_port = if local_port > 0 {
            local_port
        } else {
            self.allocate_port()
        };
        let bind_addr = format!("{}:{}", local_host, local_port);

        let rh_for_thread = remote_host.to_string();
        let rh_for_rule = remote_host.to_string();
        let stop_flag = Arc::new(Mutex::new(false));
        let stop = stop_flag.clone();
        let stop_for_entry = stop_flag.clone();

        // 启动转发线程
        let handle = thread::spawn(move || {
            // 绑定本地端口
            let listener = match TcpListener::bind(&bind_addr) {
                Ok(l) => l,
                Err(_) => return,
            };
            // 设置为非阻塞，避免 accept 阻塞整个线程
            listener.set_nonblocking(true).ok();

            loop {
                // 检查停止信号
                if *stop.lock().unwrap() {
                    return;
                }

                // 非阻塞 accept
                let stream = match listener.accept() {
                    Ok((s, _)) => s,
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                    Err(_) => return,
                };

                stream.set_nonblocking(true).ok();
                let mut loc_read = match stream.try_clone() {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                let mut loc_write = stream;

                // 创建 SSH direct-tcpip 通道（相当于 ssh -L 的效果）
                // session 为 non-blocking，channel_direct_tcpip 可能返回 EAGAIN，需重试
                let mut channel = 'create_channel: loop {
                    if *stop.lock().unwrap() {
                        return;
                    }
                    let s = session.lock().unwrap();
                    match s.channel_direct_tcpip(&rh_for_thread, remote_port, None) {
                        Ok(c) => break 'create_channel c,
                        Err(e) => {
                            drop(s); // 立即释放锁，避免阻塞其他转发线程
                            let io_err: std::io::Error = e.into();
                            if io_err.kind() == std::io::ErrorKind::WouldBlock {
                                thread::sleep(Duration::from_millis(50));
                                continue 'create_channel; // EAGAIN: 重试
                            }
                            continue; // 其它错误: 放弃本次连接
                        }
                    }
                };

                // 双向转发数据
                let stop_for_io = stop.clone();
                let mut buf = [0u8; 8192]; // 8KB 缓冲区
                loop {
                    if *stop_for_io.lock().unwrap() {
                        break;
                    }
                    // 本地 → 远程
                    match loc_read.read(&mut buf) {
                        Ok(0) => break, // 连接关闭
                        Ok(n) => {
                            let _ = channel.write_all(&buf[..n]);
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(_) => break,
                    }
                    // 远程 → 本地
                    match channel.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let _ = loc_write.write_all(&buf[..n]);
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(_) => break,
                    }
                    // 小延迟避免忙循环（busy loop）
                    thread::sleep(Duration::from_millis(10));
                }
            }
        });

        // 记录转发条目
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
            rule: rule.clone(),
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
            *entry.stop_flag.lock().unwrap() = true; // 通知线程停止
            self.stop_flags.remove(pos);
        }
        Ok(())
    }

    pub fn list_forwards(&self) -> Vec<ForwardRule> {
        self.forwards.iter().map(|f| f.rule.clone()).collect()
    }

    pub fn forward_count(&self) -> usize {
        self.forwards.len()
    }

    /// 自动分配一个可用的本地端口（10000-60000 范围内）
    fn allocate_port(&mut self) -> u16 {
        let used_ports: Vec<u16> = self.forwards.iter().map(|f| f.rule.local_port).collect();
        loop {
            let port = self.next_port;
            self.next_port += 1;
            if self.next_port > 60000 {
                self.next_port = 10000;
            } // 回绕
              // 端口未被占用且 OS 层面可用
            if !used_ports.contains(&port) && port_is_available(port) {
                return port;
            }
        }
    }
}

/// 检查端口是否可用（尝试绑定来测试）
fn port_is_available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}
