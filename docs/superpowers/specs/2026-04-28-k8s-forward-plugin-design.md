# k8s-forward 插件设计文档

## 概述

基于 Java ip-forward-plugin 架构，用 Rust 复刻 IP 转发插件。将 Nacos 服务发现替换为通过账号密码连接 Kuboard 管理端（DEX SSO 认证）获取 K8s Pod 信息，保留 SSH 端口转发和 HTTP 代理功能。

**插件 ID**: `k8s-forward`
**插件名称**: K8s IP 转发

## 核心组件

### 1. KuboardClient (reqwest)

连接 `http://10.73.64.28:8087`，通过 DEX SSO 认证后调用 K8s API。

**认证流程**:
1. GET `/kuboard/cluster` → 跟随重定向 → SSO 登录页 `/sso/auth/default?req=<id>`，提取 `req` 参数
2. POST `/sso/auth/default?req=<id>`，Content-Type: `application/x-www-form-urlencoded`，body: `login=<user>&password={"password":"<pwd>"}`
3. 解析响应错误消息：`Login error: PASS` → 成功；`Login error: MFA_REQUIRED` → 需双因子认证
4. (MFA 时) POST `/login/password`，JSON: `{username, password, passcode}`
5. 提交隐藏 form POST → 获取 session cookie → 重定向到 Kuboard

**K8s API（带 cookie）**:
- 集群列表: 待抓包确认（路径可配置）
- 命名空间: `GET /k8s-api/{cluster}/api/v1/namespaces`
- Pod 列表: `GET /k8s-api/{cluster}/api/v1/namespaces/{namespace}/pods`

401 时自动重新登录。

### 2. SshService (ssh2)

SSH 连接跳板机/堡垒机，建立本地端口转发隧道。

- `ssh2::Session` + `channel_direct_tcpip()` 实现
- 同步阻塞 API，每个转发规则独立 `std::thread` 处理 accept + 数据转发
- 端口从 10000 自动分配，释放可回收
- SSH 断开重连自动恢复所有转发规则
- 用 `std::sync::mpsc` channel 与 tokio 侧（HttpProxySvc）通信

### 3. HttpProxySvc (hyper)

HTTP 反向代理，通过 Host header 匹配路由到 SSH 转发端口。

- hyper 基于 tokio 异步
- 维护域名→目标映射表: `{domain}` → `127.0.0.1:{localPort}`
- 代理端口默认 80（可配置）
- 透传请求头、请求体、响应头、响应体
- 域名生成规则: `{pod-name}-{container-name}.svc`

## API 设计 (handle_call)

24 个方法，5 组：

| 组 | 方法 | 说明 |
|---|---|---|
| SSH | `ssh_connect`, `ssh_disconnect`, `ssh_status` | SSH 连接管理 |
| Rule | `list_forward_rules`, `add_forward_rule`, `update_forward_rule`, `remove_forward_rule`, `import_rules`, `export_rules` | 手动规则 CRUD |
| K8s | `kuboard_login`, `kuboard_mfa`, `kuboard_logout`, `kuboard_status`, `list_clusters`, `list_namespaces`, `list_pods` | Kuboard 连接 & 发现 |
| K8s | `forward_pod`, `unforward_pod`, `list_k8s_forwards` | K8s 转发操作 |
| Proxy | `proxy_start`, `proxy_stop`, `proxy_status`, `list_proxy_mappings`, `update_proxy_mapping` | HTTP 代理 |
| Cfg | `get_config`, `reset_config` | 配置管理 |

## 前端 UI

3 个 Tab，React + TypeScript + Vite，iframe srcdoc 渲染。

### Tab 1 — SSH 端口转发
- SSH 连接表单（host, port, username, password）+ 连接/断开按钮
- 转发规则表格（名称, 本地地址, 本地端口, 远程地址, 远程端口, 操作）
- [+ 添加规则] [导入 JSON] [导出 JSON]
- 规则可编辑（行内编辑或弹窗）

### Tab 2 — K8s 服务转发
- Kuboard 登录表单（url, username, password）+ MFA 弹窗
- 集群/命名空间下拉选择
- Pod 列表（Pod 名, IP, 容器, 端口, 状态, 操作）
- 搜索过滤 + 刷新
- 已转发列表（域名, 本地端口, 目标, 操作）
- 域名可编辑

### Tab 3 — HTTP 代理
- 代理端口 + 启动/停止按钮 + 状态
- 域名路由映射表（域名, 目标地址, 状态）
- 域名可编辑
- hosts 提示

## 数据模型

```rust
struct ForwardRule {
    id: String,
    name: String,
    local_host: String,
    local_port: u16,
    remote_host: String,
    remote_port: u16,
    rule_type: RuleType,  // Manual | K8s
    // K8s 特有
    cluster: Option<String>,
    namespace: Option<String>,
    pod_name: Option<String>,
    container_name: Option<String>,
}

struct ProxyMapping {
    domain: String,
    target: String,       // "127.0.0.1:10000"
    rule_id: String,
    editable: bool,
}
```

## 配置持久化

PluginStorage → `~/.worktools/history/plugins/k8s-forward.json`

SSH 和 Kuboard 密码使用 AES-256 ECB + PKCS7 加密存储（复用 password-manager 的 crypto 方案，固定内部密钥）。

## 线程模型

| 组件 | 模型 | 说明 |
|---|---|---|
| KuboardClient | tokio async | reqwest 异步 |
| SshService | 独立 std::thread | ssh2 同步阻塞，每转发一线程 |
| HttpProxySvc | tokio task | hyper 基于 tokio |
| Plugin runtime | tokio | 参考 db-doc，init 时创建 runtime |

SshService 通过 `std::sync::mpsc` channel 与 tokio 侧通信。

## 导入导出

前端侧实现（参考 db-router）：
- **导入**: 创建隐藏 `<input type="file">`，读取 JSON → `call("import_rules", {rules})`
- **导出**: `call("export_rules")` → JSON → `showSaveFilePicker` / Blob download

## 依赖

| Crate | 用途 |
|---|---|
| `ssh2` | SSH 连接 + 端口转发 |
| `hyper` + `hyper-util` | HTTP 代理服务器 |
| `reqwest` | Kuboard API 客户端 |
| `tokio` | 异步运行时 |
| `serde` / `serde_json` | 序列化 |
| `aes` + `sha2` + `hex` | 密码加密 |
| `uuid` | 转发规则 ID 生成 |
| `worktools-plugin-api` | Plugin trait |

## K8s API 路径配置化

Kuboard 版本不同可能导致 API 路径变化。以下路径为默认值，可在插件配置中编辑：
- 登录前缀: `/sso/auth/default`
- 集群列表: `/kuboard/api/clusters`
- 命名空间: `/k8s-api/{cluster}/api/v1/namespaces`
- Pod 列表: `/k8s-api/{cluster}/api/v1/namespaces/{namespace}/pods`

## 构建产物

```
plugins/k8s-forward/
  Cargo.toml
  manifest.json
  src/
    lib.rs              # Plugin trait 实现 + handle_call 路由
    kuboard_client.rs   # KuboardClient
    ssh_service.rs      # SshService
    http_proxy.rs       # HttpProxySvc
    crypto.rs           # 密码加密（参考 password-manager）
    models.rs           # 数据模型
  frontend/
    (React + Vite 标准结构)
  assets/
    index.html
    main.js
    styles.css
```
