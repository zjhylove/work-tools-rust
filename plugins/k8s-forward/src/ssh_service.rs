use anyhow::{Result, anyhow};
use ssh2::Session;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::net::TcpListener;
use crate::models::{ForwardRule, RuleType};

pub struct SshService {
    session: Option<Session>,
    tcp_stream: Option<TcpStream>,
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
            tcp_stream: None,
            forwards: vec![],
            next_port: 10000,
            threads: vec![],
            stop_flags: vec![],
        }
    }

    pub fn is_connected(&self) -> bool {
        self.session.as_ref().map(|s| s.authenticated()).unwrap_or(false)
    }

    pub fn connect(&mut self, host: &str, port: u16, username: &str, password: &str) -> Result<()> {
        let addr = format!("{}:{}", host, port);
        let tcp = TcpStream::connect(&addr)?;
        tcp.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp.try_clone()?);
        session.handshake()?;
        session.userauth_password(username, password)?;
        if !session.authenticated() {
            return Err(anyhow!("SSH 认证失败"));
        }
        self.session = Some(session);
        self.tcp_stream = Some(tcp);
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
        self.tcp_stream = None;
    }

    pub fn add_forward(&mut self, local_host: &str, remote_host: &str, remote_port: u16) -> Result<u16> {
        let local_port = self.allocate_port();
        let bind_addr = format!("{}:{}", local_host, local_port);

        let stop_flag = Arc::new(Mutex::new(false));
        let stop = stop_flag.clone();
        let stop_for_entry = stop_flag.clone();
        let remote = format!("{}:{}", remote_host, remote_port);

        let handle = thread::spawn(move || {
            let listener = match TcpListener::bind(&bind_addr) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("无法绑定 {}: {}", bind_addr, e);
                    return;
                }
            };
            listener.set_nonblocking(true).ok();

            loop {
                if *stop.lock().unwrap() { break; }
                match listener.accept() {
                    Ok((local_stream, _)) => {
                        let remote_clone = remote.clone();
                        thread::spawn(move || {
                            if let Ok(mut remote_stream) = TcpStream::connect(&remote_clone) {
                                let mut local_clone = local_stream.try_clone().unwrap();
                                let _ = std::io::copy(&mut local_clone, &mut remote_stream);
                            }
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(_) => break,
                }
            }
        });

        let rule = ForwardRule {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!("forward-{}", local_port),
            local_host: local_host.to_string(),
            local_port,
            remote_host: remote_host.to_string(),
            remote_port,
            rule_type: RuleType::Manual,
            cluster: None,
            namespace: None,
            pod_name: None,
            container_name: None,
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

    pub fn forward_count(&self) -> usize {
        self.forwards.len()
    }

    fn allocate_port(&mut self) -> u16 {
        let used_ports: Vec<u16> = self.forwards.iter().map(|f| f.rule.local_port).collect();
        loop {
            let port = self.next_port;
            self.next_port += 1;
            if self.next_port > 60000 {
                self.next_port = 10000;
            }
            if !used_ports.contains(&port) && port_is_available(port) {
                return port;
            }
        }
    }
}

fn port_is_available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}
