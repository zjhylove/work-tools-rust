use anyhow::{Result, anyhow};
use ssh2::Session;
use std::io::{Read, Write};
use std::net::{TcpStream, TcpListener};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use crate::models::{ForwardRule, RuleType};

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
        self.session.as_ref().map(|s| s.lock().unwrap().authenticated()).unwrap_or(false)
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
        self.session = Some(Arc::new(Mutex::new(session)));
        Ok(())
    }

    pub fn disconnect(&mut self) {
        for flag in &self.stop_flags { *flag.lock().unwrap() = true; }
        for handle in self.threads.drain(..) { let _ = handle.join(); }
        self.stop_flags.clear();
        self.forwards.clear();
        self.session = None;
    }

    pub fn add_forward(&mut self, local_host: &str, remote_host: &str, remote_port: u16, local_port: u16) -> Result<u16> {
        let session = self.session.clone().ok_or_else(|| anyhow!("SSH 未连接"))?;
        let local_port = if local_port > 0 { local_port } else { self.allocate_port() };
        let bind_addr = format!("{}:{}", local_host, local_port);
        let remote_host = remote_host.to_string();

        let rh_for_thread = remote_host.clone();
        let rh_for_rule = remote_host;
        let stop_flag = Arc::new(Mutex::new(false));
        let stop = stop_flag.clone();
        let stop_for_entry = stop_flag.clone();

        let handle = thread::spawn(move || {
            let listener = match TcpListener::bind(&bind_addr) {
                Ok(l) => l,
                Err(_) => return,
            };
            listener.set_nonblocking(true).ok();

            loop {
                if *stop.lock().unwrap() { return; }
                let stream = match listener.accept() {
                    Ok((s, _)) => s,
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(100)); continue;
                    }
                    Err(_) => return,
                };

                stream.set_nonblocking(true).ok();
                let mut loc_read = match stream.try_clone() { Ok(r) => r, Err(_) => continue };
                let mut loc_write = stream;

                let s = session.lock().unwrap();
                let mut channel = match s.channel_direct_tcpip(&rh_for_thread, remote_port, None) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                s.set_blocking(false);

                let stop_for_io = stop.clone();
                let mut buf = [0u8; 8192];
                loop {
                    if *stop_for_io.lock().unwrap() { break; }
                    match loc_read.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => { let _ = channel.write_all(&buf[..n]); }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(_) => break,
                    }
                    match channel.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => { let _ = loc_write.write_all(&buf[..n]); }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(_) => break,
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                s.set_blocking(true);
            }
        });

        let rule = ForwardRule {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!("forward-{}", local_port),
            local_host: local_host.to_string(), local_port,
            remote_host: rh_for_rule, remote_port,
            rule_type: RuleType::Manual,
            cluster: None, namespace: None, pod_name: None, container_name: None,
        };

        self.threads.push(handle);
        self.stop_flags.push(stop_flag);
        self.forwards.push(ForwardEntry { rule: rule.clone(), stop_flag: stop_for_entry });

        Ok(local_port)
    }

    pub fn remove_forward(&mut self, local_port: u16) -> Result<()> {
        if let Some(pos) = self.forwards.iter().position(|f| f.rule.local_port == local_port) {
            let entry = self.forwards.remove(pos);
            *entry.stop_flag.lock().unwrap() = true;
            self.stop_flags.remove(pos);
        }
        Ok(())
    }

    pub fn list_forwards(&self) -> Vec<ForwardRule> {
        self.forwards.iter().map(|f| f.rule.clone()).collect()
    }

    pub fn forward_count(&self) -> usize { self.forwards.len() }

    fn allocate_port(&mut self) -> u16 {
        let used_ports: Vec<u16> = self.forwards.iter().map(|f| f.rule.local_port).collect();
        loop {
            let port = self.next_port;
            self.next_port += 1;
            if self.next_port > 60000 { self.next_port = 10000; }
            if !used_ports.contains(&port) && port_is_available(port) { return port; }
        }
    }
}

fn port_is_available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}
