# k8s-forward SSH 自动重连实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 k8s-forward 插件增加 SSH 连接断开后的自动重连能力，使用心跳线程检测断连，指数退避重试，并在重连成功后恢复所有转发规则。

**Architecture:** 在 `SshService` 中新增心跳守护线程，每 15 秒检测 SSH session 存活状态（连续 2 次失败判定断连）。断连后自动触发重连流程（指数退避 2s→60s，最多 10 次），重连成功后从持久化的 forward_rules 恢复转发。前端通过 `ssh_status` 轮询获取 `reconnecting` 状态并展示。

**Tech Stack:** Rust (ssh2, std::thread, std::sync) | TypeScript/React (前端状态展示)

---

## 文件变更清单

| 文件 | 操作 | 职责 |
|------|------|------|
| `plugins/k8s-forward/src/models.rs` | 修改 | 新增 `SshConnectionState` 枚举、`ReconnectInfo` 结构体、扩展 `SshStatus` |
| `plugins/k8s-forward/src/ssh_service.rs` | 修改 | 新增 `last_connect_params`、心跳线程、重连逻辑、`reconnect_state` 管理 |
| `plugins/k8s-forward/src/lib.rs` | 修改 | 新增 `ssh_reconnect` handler、变更 `ssh_status` 返回值、`ssh_disconnect` 停止心跳、`ssh_connect` 启动心跳 |
| `plugins/k8s-forward/frontend/src/types.ts` | 修改 | 扩展 `SshStatus` 接口 |
| `plugins/k8s-forward/frontend/src/components/TabSshForward.tsx` | 修改 | 重连状态展示、重连按钮、定期轮询 `ssh_status` |
| `plugins/k8s-forward/frontend/src/components/TabK8sForward.tsx` | 修改 | 重连状态展示 |

---

### Task 1: 扩展 models.rs 数据模型

**Files:**
- Modify: `plugins/k8s-forward/src/models.rs`

- [ ] **Step 1: 新增 `SshConnectionState` 枚举和 `ReconnectInfo` 结构体，扩展 `SshStatus`**

在 `models.rs` 文件末尾（`K8sForwardInfo` 之后）添加：

```rust
/// SSH 连接状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SshConnectionState {
    Connected,
    Disconnected,
    Reconnecting,
}

/// 重连元信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconnectInfo {
    pub retry_count: u32,
    pub max_retries: u32,
    pub next_retry_at: u64, // Unix 时间戳（秒）
}
```

修改 `SshStatus` 结构体：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshStatus {
    pub connected: bool,
    pub host: Option<String>,
    pub port: Option<u16>,
    #[serde(default = "default_connection_state")]
    pub status: SshConnectionState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconnect_info: Option<ReconnectInfo>,
}

fn default_connection_state() -> SshConnectionState {
    SshConnectionState::Disconnected
}
```

- [ ] **Step 2: 运行 cargo check 验证编译**

Run: `cargo check -p k8s-forward`
Expected: 编译成功（新类型暂未使用，但不应出错）

- [ ] **Step 3: Commit**

```bash
git add plugins/k8s-forward/src/models.rs
git commit -m "feat(k8s-forward): add SshConnectionState and ReconnectInfo models"
```

---

### Task 2: 扩展 SshService — 连接参数缓存与重连状态

**Files:**
- Modify: `plugins/k8s-forward/src/ssh_service.rs`

- [ ] **Step 1: 新增连接参数结构体和重连状态字段**

在 `ssh_service.rs` 顶部 import 区域，添加：

```rust
use crate::models::{ForwardRule, ReconnectInfo, RuleType, SshConnectionState};
```

在 `ActiveConn` 结构体之前添加：

```rust
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
    abort: bool, // 用户手动取消标志
}
```

修改 `SshService` 结构体，添加新字段：

```rust
pub struct SshService {
    session: Option<Arc<Mutex<Session>>>,
    forwards: Vec<ForwardEntry>,
    next_port: u16,
    threads: Vec<thread::JoinHandle<()>>,
    stop_flags: Vec<Arc<Mutex<bool>>>,
    // 新增字段
    connect_params: Option<ConnectParams>,
    heartbeat_stop: Arc<Mutex<bool>>,
    heartbeat_thread: Option<thread::JoinHandle<()>>,
    reconnect_stop: Arc<Mutex<bool>>,
    reconnect_state: Option<ReconnectState>,
    reconnect_thread: Option<thread::JoinHandle<()>>,
}
```

修改 `SshService::new()`：

```rust
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
        reconnect_state: None,
        reconnect_thread: None,
    }
}
```

- [ ] **Step 2: 添加连接状态查询方法**

在 `impl SshService` 中，在 `is_connected()` 方法之后添加：

```rust
pub fn connection_state(&self) -> SshConnectionState {
    if self.reconnect_state.is_some() {
        SshConnectionState::Reconnecting
    } else if self.is_connected() {
        SshConnectionState::Connected
    } else {
        SshConnectionState::Disconnected
    }
}

pub fn get_reconnect_info(&self) -> Option<ReconnectInfo> {
    self.reconnect_state.as_ref().map(|rs| ReconnectInfo {
        retry_count: rs.retry_count,
        max_retries: rs.max_retries,
        next_retry_at: (std::time::SystemTime::now() + rs.next_retry_at.duration_since(std::time::Instant::now()).unwrap_or_default())
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    })
}
```

- [ ] **Step 3: 修改 `connect` 方法保存连接参数**

修改 `connect` 方法，在 `session.set_blocking(false);` 之后、`self.session = Some(...)` 之前添加：

```rust
self.connect_params = Some(ConnectParams {
    host: host.to_string(),
    port,
    username: username.to_string(),
    password: password.to_string(),
});
```

- [ ] **Step 4: 运行 cargo check**

Run: `cargo check -p k8s-forward`
Expected: 编译通过（新增字段和方法暂未在 call chain 中使用）

- [ ] **Step 5: Commit**

```bash
git add plugins/k8s-forward/src/ssh_service.rs
git commit -m "feat(k8s-forward): add connect params cache and reconnect state fields"
```

---

### Task 3: 实现心跳检测线程

**Files:**
- Modify: `plugins/k8s-forward/src/ssh_service.rs`

- [ ] **Step 1: 实现 `start_heartbeat` 和 `stop_heartbeat` 方法**

在 `impl SshService` 中，`bind_auto_port` 方法之前添加：

```rust
/// 启动心跳检测线程
pub fn start_heartbeat(&mut self) {
    // 先停止旧的心跳线程
    self.stop_heartbeat();

    *self.heartbeat_stop.lock().unwrap() = false;
    let stop = self.heartbeat_stop.clone();
    let session_check = self.session.clone();

    let handle = thread::spawn(move || {
        let mut fail_count = 0u32;
        loop {
            if *stop.lock().unwrap() {
                return;
            }
            thread::sleep(Duration::from_secs(15));

            if *stop.lock().unwrap() {
                return;
            }

            let alive = session_check
                .as_ref()
                .map(|s| s.lock().unwrap().is_connected())
                .unwrap_or(false);

            if alive {
                fail_count = 0;
            } else {
                fail_count += 1;
                if fail_count >= 2 {
                    tracing::warn!("SSH 心跳检测失败 {} 次，判定连接已断开", fail_count);
                    return; // 退出心跳线程，由调用方检查线程退出触发重连
                }
            }
        }
    });

    self.heartbeat_thread = Some(handle);
}

/// 停止心跳检测线程
pub fn stop_heartbeat(&mut self) {
    *self.heartbeat_stop.lock().unwrap() = true;
    if let Some(handle) = self.heartbeat_thread.take() {
        let _ = handle.join();
    }
}

/// 检查心跳线程是否仍在运行（已退出表示检测到断连）
pub fn heartbeat_exited(&self) -> bool {
    self.heartbeat_thread
        .as_ref()
        .map(|h| h.is_finished())
        .unwrap_or(true)
}
```

- [ ] **Step 2: 运行 cargo check**

Run: `cargo check -p k8s-forward`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add plugins/k8s-forward/src/ssh_service.rs
git commit -m "feat(k8s-forward): implement heartbeat thread for SSH connection monitoring"
```

---

### Task 4: 实现自动重连逻辑

**Files:**
- Modify: `plugins/k8s-forward/src/ssh_service.rs`

- [ ] **Step 1: 实现 `start_reconnect`、`stop_reconnect`、`restore_forwards` 方法**

在 `impl SshService` 中，`bind_auto_port` 方法之前添加：

```rust
/// 启动自动重连
pub fn start_reconnect(&mut self) {
    self.stop_reconnect();

    let params = match &self.connect_params {
        Some(p) => ConnectParams {
            host: p.host.clone(),
            port: p.port,
            username: p.username.clone(),
            password: p.password.clone(),
        },
        None => return,
    };

    self.reconnect_state = Some(ReconnectState {
        retry_count: 0,
        max_retries: 10,
        next_retry_at: std::time::Instant::now(),
        abort: false,
    });

    let stop = self.reconnect_stop.clone();
    let session_slot = self.session.clone();

    // 注意：重连需要访问 self 的多个字段，这里通过参数传递必要数据
    let host = params.host.clone();
    let port = params.port;
    let username = params.username.clone();
    let password = params.password.clone();

    let handle = thread::spawn(move || {
        let mut delay = Duration::from_secs(2);
        let max_delay = Duration::from_secs(60);

        for attempt in 1..=10u32 {
            if *stop.lock().unwrap() {
                tracing::info!("SSH 重连已取消");
                return None;
            }

            tracing::info!("SSH 重连尝试 {}/10，{} 秒后执行...", attempt, delay.as_secs());
            let sleep_until = std::time::Instant::now() + delay;
            while std::time::Instant::now() < sleep_until {
                if *stop.lock().unwrap() {
                    tracing::info!("SSH 重连已取消");
                    return None;
                }
                thread::sleep(Duration::from_secs(1));
            }

            match Self::try_connect(&host, port, &username, &password) {
                Ok(session) => {
                    tracing::info!("SSH 重连成功（第 {} 次尝试）", attempt);
                    *session_slot.lock().unwrap() = Some(session);
                    return Some(());
                }
                Err(e) => {
                    tracing::warn!("SSH 重连失败（第 {} 次）: {}", attempt, e);
                }
            }

            delay = std::cmp::min(delay * 2, max_delay);
        }

        tracing::error!("SSH 重连失败，已耗尽 10 次重试");
        None
    });

    self.reconnect_thread = Some(handle);
}

/// 停止重连
pub fn stop_reconnect(&mut self) {
    *self.reconnect_stop.lock().unwrap() = true;
    if let Some(handle) = self.reconnect_thread.take() {
        let _ = handle.join();
    }
    *self.reconnect_stop.lock().unwrap() = false;
    self.reconnect_state = None;
}

/// 尝试建立 SSH 连接（静态方法，供重连线程使用）
fn try_connect(host: &str, port: u16, username: &str, password: &str) -> Result<Arc<Mutex<Session>>> {
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
    Ok(Arc::new(Mutex::new(session)))
}
```

- [ ] **Step 2: 添加 `reconnect_finished` 检查方法**

```rust
/// 检查重连线程是否完成，返回 Some(true) 表示重连成功，Some(false) 表示失败，None 表示仍在进行
pub fn check_reconnect_result(&mut self) -> Option<bool> {
    let handle = self.reconnect_thread.as_ref()?;
    if !handle.is_finished() {
        return None;
    }
    // 线程已结束，join 获取结果
    let handle = self.reconnect_thread.take().unwrap();
    let result = handle.join().ok().flatten();
    self.reconnect_state = None;
    Some(result.is_some())
}
```

- [ ] **Step 3: 修改 `disconnect` 方法清理所有线程**

替换现有 `disconnect` 方法：

```rust
pub fn disconnect(&mut self) {
    // 停止重连
    self.stop_reconnect();
    // 停止心跳
    self.stop_heartbeat();

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
```

- [ ] **Step 4: 运行 cargo check**

Run: `cargo check -p k8s-forward`
Expected: 编译通过

- [ ] **Step 5: Commit**

```bash
git add plugins/k8s-forward/src/ssh_service.rs
git commit -m "feat(k8s-forward): implement auto-reconnect with exponential backoff"
```

---

### Task 5: 修改 lib.rs — 集成心跳、重连和 ssh_status 变更

**Files:**
- Modify: `plugins/k8s-forward/src/lib.rs`

- [ ] **Step 1: 修改 `handle_ssh_connect` — 启动心跳**

在 `handle_ssh_connect` 方法中，找到 `ssh.connect(host, port, username, password)?;` 调用，在其之后（恢复转发规则的循环之前）添加：

```rust
ssh.start_heartbeat();
```

- [ ] **Step 2: 修改 `handle_ssh_disconnect` — 停止心跳和重连**

`handle_ssh_disconnect` 已经调用 `self.ssh.lock().unwrap().disconnect()`，而 `disconnect()` 内部已调用 `stop_reconnect()` 和 `stop_heartbeat()`，无需额外修改。

- [ ] **Step 3: 修改 `handle_ssh_status` — 返回新状态**

替换 `handle_ssh_status` 方法：

```rust
fn handle_ssh_status(&self) -> Result<Value> {
    let mut ssh = self.ssh.lock().unwrap();
    let data = self.load_data()?;

    // 检查重连线程结果
    let reconnect_result = ssh.check_reconnect_result();

    // 如果重连成功，恢复转发规则并启动心跳
    if reconnect_result == Some(true) {
        let mut restored = 0;
        let mut data_inner = self.load_data()?;
        for rule in data_inner.forward_rules.iter_mut() {
            match ssh.add_forward(
                &rule.local_host,
                &rule.remote_host,
                rule.remote_port,
                rule.local_port,
            ) {
                Ok(assigned) => {
                    if rule.local_port == 0 {
                        rule.local_port = assigned;
                    }
                    restored += 1;
                }
                Err(e) => tracing::warn!("重连后恢复转发规则失败 [{}]: {}", rule.name, e),
            }
        }
        if restored > 0 {
            self.save_data(&data_inner)?;
        }
        ssh.start_heartbeat();
        tracing::info!("SSH 重连成功，已恢复 {} 条转发规则", restored);
    }

    let state = ssh.connection_state();
    let reconnect_info = ssh.get_reconnect_info();

    let status = SshStatus {
        connected: state == SshConnectionState::Connected,
        host: data.ssh.as_ref().map(|s| s.host.clone()),
        port: data.ssh.as_ref().map(|s| s.port),
        status: state,
        reconnect_info,
    };
    Ok(serde_json::to_value(status)?)
}
```

- [ ] **Step 4: 新增 `handle_ssh_reconnect` 方法**

在 `handle_ssh_disconnect` 方法之后添加：

```rust
fn handle_ssh_reconnect(&self) -> Result<Value> {
    let mut ssh = self.ssh.lock().unwrap();
    if ssh.is_connected() {
        return Err(anyhow::anyhow!("SSH 已连接，无需重连"));
    }
    if ssh.connect_params.is_none() {
        return Err(anyhow::anyhow!("没有保存的连接参数，请使用 ssh_connect"));
    }
    // 停止可能正在进行的重连
    ssh.stop_reconnect();
    ssh.start_reconnect();
    Ok(json!({"success": true, "message": "开始重连..."}))
}
```

- [ ] **Step 5: 在 `handle_call` 的 match 中注册新方法**

在 `"ssh_status"` 分支之后添加：

```rust
"ssh_reconnect" => dispatch!(self.handle_ssh_reconnect()),
```

- [ ] **Step 6: 添加 models import**

在 `lib.rs` 顶部的 `use models::*;` 已经是 glob import，新增的类型自动可见。确认 `use models::*;` 存在即可。

- [ ] **Step 7: 运行 cargo check**

Run: `cargo check -p k8s-forward`
Expected: 编译通过

- [ ] **Step 8: Commit**

```bash
git add plugins/k8s-forward/src/lib.rs
git commit -m "feat(k8s-forward): integrate heartbeat, reconnect into ssh_status and add ssh_reconnect command"
```

---

### Task 6: 集成心跳退出检测到自动重连触发

**Files:**
- Modify: `plugins/k8s-forward/src/lib.rs`

当前的 `handle_ssh_status` 只在轮询时检查重连结果。还需要在轮询时检测心跳线程退出（表示断连），触发自动重连。

- [ ] **Step 1: 在 `handle_ssh_status` 中增加心跳退出检测**

在 `handle_ssh_status` 方法中，`let reconnect_result = ssh.check_reconnect_result();` 之前添加心跳退出检测：

```rust
// 检查心跳线程是否退出（退出表示检测到断连）
if ssh.heartbeat_exited() && ssh.reconnect_state.is_none() && ssh.connect_params.is_some() {
    tracing::warn!("SSH 心跳检测到断连，启动自动重连");
    ssh.start_reconnect();
}
```

- [ ] **Step 2: 运行 cargo check**

Run: `cargo check -p k8s-forward`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add plugins/k8s-forward/src/lib.rs
git commit -m "feat(k8s-forward): trigger auto-reconnect when heartbeat detects disconnect"
```

---

### Task 7: 更新前端 types.ts

**Files:**
- Modify: `plugins/k8s-forward/frontend/src/types.ts`

- [ ] **Step 1: 扩展 SshStatus 接口**

替换 `SshStatus` 接口：

```typescript
export type SshConnectionState = "Connected" | "Disconnected" | "Reconnecting";

export interface ReconnectInfo {
  retry_count: number;
  max_retries: number;
  next_retry_at: number; // Unix 时间戳（秒）
}

export interface SshStatus {
  connected: boolean;
  host?: string;
  port?: number;
  status: SshConnectionState;
  reconnect_info?: ReconnectInfo;
}
```

- [ ] **Step 2: Commit**

```bash
git add plugins/k8s-forward/frontend/src/types.ts
git commit -m "feat(k8s-forward): extend SshStatus type with reconnect state"
```

---

### Task 8: 更新 TabSshForward.tsx — 重连状态展示与轮询

**Files:**
- Modify: `plugins/k8s-forward/frontend/src/components/TabSshForward.tsx`

- [ ] **Step 1: 添加 ssh_status 定期轮询**

在 `TabSshForward` 组件中，现有 `useEffect` 之后添加新的 `useEffect` 用于定期轮询：

```typescript
// 定期轮询 SSH 状态（检测重连状态变化）
useEffect(() => {
  if (!sshStatus.connected && sshStatus.status !== "Reconnecting") return;
  const timer = setInterval(loadStatus, 5000);
  return () => clearInterval(timer);
}, [sshStatus.connected, sshStatus.status]);
```

- [ ] **Step 2: 修改状态显示区域**

找到状态显示的 JSX（包含 `status-dot` 的区域），替换为：

```tsx
<span className={`status-dot ${
  sshStatus.status === "Connected" ? "online" :
  sshStatus.status === "Reconnecting" ? "reconnecting" :
  "offline"
}`}></span>
{sshStatus.status === "Connected" && `已连接 → ${sshStatus.host}:${sshStatus.port}`}
{sshStatus.status === "Reconnecting" && `重连中 (第 ${sshStatus.reconnect_info?.retry_count ?? 0}/${sshStatus.reconnect_info?.max_retries ?? 10} 次)...`}
{sshStatus.status === "Disconnected" && (sshStatus.reconnect_info?.retry_count === sshStatus.reconnect_info?.max_retries ? "连接已断开，请手动重连" : "未连接")}
```

- [ ] **Step 3: 添加重连按钮**

在状态显示区域之后，连接/断开按钮区域，修改按钮逻辑：

找到 `handleConnect` / `handleDisconnect` 按钮区域，将连接按钮逻辑改为：

```tsx
{sshStatus.status === "Connected" ? (
  <button className="wt-btn--danger" onClick={handleDisconnect}>断开</button>
) : sshStatus.status === "Reconnecting" ? (
  <button className="wt-btn--secondary" disabled>重连中...</button>
) : (
  <button className="wt-btn--primary" onClick={handleConnect}>连接</button>
)}
{sshStatus.status === "Disconnected" && sshStatus.reconnect_info?.retry_count === sshStatus.reconnect_info?.max_retries && (
  <button className="wt-btn--secondary" onClick={async () => { await call("ssh_reconnect"); loadStatus(); }} style={{ marginLeft: 8 }}>
    重新连接
  </button>
)}
```

- [ ] **Step 4: 添加 reconnecting 状态 CSS 类**

在 `TabSshForward.tsx` 对应的样式文件（或内联样式）中，`status-dot.reconnecting` 使用橙色脉冲动画：

```css
.status-dot.reconnecting {
  background: var(--warning, #f0ad4e);
  animation: pulse 1.5s infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
```

注意：需确认样式文件位置。如果组件使用 CSS 文件，在对应文件中添加；如果使用内联样式，则改为 inline style 实现脉冲效果。

- [ ] **Step 5: Commit**

```bash
git add plugins/k8s-forward/frontend/src/components/TabSshForward.tsx
git commit -m "feat(k8s-forward): add reconnect status display and polling in SSH tab"
```

---

### Task 9: 更新 TabK8sForward.tsx — 重连状态展示

**Files:**
- Modify: `plugins/k8s-forward/frontend/src/components/TabK8sForward.tsx`

- [ ] **Step 1: 添加 SSH 状态查询和定期轮询**

在 `TabK8sForward` 组件中添加 sshStatus state 和轮询：

```typescript
const [sshStatus, setSshStatus] = useState<SshStatus>({ connected: false, status: "Disconnected" });

const loadSshStatus = async () => {
  try {
    setSshStatus(await call("ssh_status") as SshStatus);
  } catch { /* ignore */ }
};

// 定期轮询 SSH 状态
useEffect(() => {
  loadSshStatus();
  const timer = setInterval(loadSshStatus, 5000);
  return () => clearInterval(timer);
}, []);
```

- [ ] **Step 2: 在 K8s 转发区域显示 SSH 重连状态提示**

在转发列表区域（`forwards.rules` 渲染之前）添加断连提示：

```tsx
{sshStatus.status === "Reconnecting" && (
  <div className="reconnect-banner">
    SSH 重连中 (第 {sshStatus.reconnect_info?.retry_count ?? 0}/{sshStatus.reconnect_info?.max_retries ?? 10} 次)...
  </div>
)}
{sshStatus.status === "Disconnected" && sshStatus.reconnect_info?.retry_count === sshStatus.reconnect_info?.max_retries && (
  <div className="reconnect-banner error">
    SSH 连接已断开，请前往 SSH端口转发 页面重新连接
  </div>
)}
```

添加对应样式：

```css
.reconnect-banner {
  padding: 8px 12px;
  background: var(--warning-bg, rgba(240, 173, 78, 0.1));
  color: var(--warning, #f0ad4e);
  border-radius: 6px;
  margin-bottom: 12px;
  font-size: 13px;
}
.reconnect-banner.error {
  background: var(--danger-bg, rgba(220, 53, 69, 0.1));
  color: var(--danger, #dc3545);
}
```

- [ ] **Step 3: Commit**

```bash
git add plugins/k8s-forward/frontend/src/components/TabK8sForward.tsx
git commit -m "feat(k8s-forward): show SSH reconnect status in K8s tab"
```

---

### Task 10: 集成测试验证

**Files:**
- No new files (manual testing)

- [ ] **Step 1: 运行 cargo check 全量检查**

Run: `cargo check -p k8s-forward`
Expected: 编译通过，无警告

- [ ] **Step 2: 运行 cargo test**

Run: `cargo test -p k8s-forward`
Expected: 所有测试通过

- [ ] **Step 3: 运行前端类型检查**

Run: `cd plugins/k8s-forward/frontend && npx tsc --noEmit`
Expected: 类型检查通过

- [ ] **Step 4: 启动开发服务器进行手动验证**

Run: `cd tauri-app && npm run tauri dev`

验证点：
1. 连接 SSH → 查看心跳线程是否启动（控制台日志）
2. 手动断开 SSH 服务端 → 观察是否自动重连
3. 重连过程中前端状态展示是否正确
4. 重连成功后转发规则是否恢复
5. 重试耗尽后前端是否显示断连提示和重连按钮
6. 手动点击重连按钮是否正常工作

- [ ] **Step 5: Commit 所有修复（如有）**
