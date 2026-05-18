# K8s IP 转发（k8s-forward）

> 通过 Kuboard 发现 K8s Pod，使用 SSH 隧道 + HTTP 代理将本地流量转发到远程集群 Pod。

## 功能特性

- SSH 端口转发：手动创建 SSH 隧道，将远程主机端口映射到本地
- Kuboard 集成：登录 Kuboard，浏览集群、命名空间、Pod 列表
- K8s Pod 转发：一键将 Pod 端口转发到本地，自动创建 SSH 隧道
- HTTP 反向代理：本地 HTTP 代理服务器，通过域名映射将请求转发到对应 Pod
- 转发规则管理：增删改查、导入导出（JSON 文件）
- 凭据加密存储：SSH 密码和 Kuboard 密码使用 AES 加密
- MFA 双因子认证：支持 Kuboard 的 MFA 验证
- 自动恢复转发规则：SSH 重连后自动恢复之前保存的转发
- 转发规则验证：检查已保存的 K8s 转发规则对应的 Pod 是否仍然存在

## 使用方法

### 基本操作

插件分为三个功能标签页：**SSH 端口转发**、**K8s 服务转发**、**HTTP 代理**。

**SSH 端口转发**：

1. 填写 SSH 跳板机的地址、端口、用户名和密码，点击「连接」
2. 连接成功后自动恢复之前保存的转发规则
3. 点击「添加规则」创建新的端口映射（本地地址:端口 -> 远程地址:端口）
4. 支持导入/导出规则（JSON 文件）

**K8s 服务转发**：

1. 填写 Kuboard 地址、用户名和密码，点击「登录」
2. 如果启用了 MFA，输入验证码完成二次验证
3. 选择集群和命名空间，加载 Pod 列表
4. 在 Pod 列表中点击「转发」，自动创建 SSH 隧道和代理映射
5. 已转发的 Pod 显示在「已转发列表」中，可编辑或取消

**HTTP 代理**：

1. 设置代理端口（默认 80），点击「启动」
2. 启动后显示代理映射表（Pod 名称 -> 本地端口）
3. 可编辑 Pod 地址映射
4. 浏览器配置代理后通过 Pod 名称或 Pod 地址访问 K8s 服务

### 配置项

| 参数 | 说明 | 默认值 |
|------|------|--------|
| SSH 主机地址 | SSH 跳板机 IP | -- |
| SSH 端口 | SSH 端口 | 22 |
| Kuboard 地址 | Kuboard 管理界面 URL | -- |
| HTTP 代理端口 | 本地反向代理监听端口 | 80 |

## 技术实现

### 后端（Rust）

这是整个项目中最复杂的插件，集成了多种技术栈。

**模块结构**：

```
src/
├── lib.rs              # 插件主入口，handle_call 27 个方法分发
├── crypto.rs           # AES 密码加密/解密
├── models.rs           # 数据模型（ForwardRule, ProxyMapping, PodInfo 等）
├── ssh_service.rs      # SSH 连接管理 + 端口转发（ssh2 crate）
├── kuboard_client.rs   # Kuboard API 客户端（reqwest + cookie）
└── http_proxy.rs       # HTTP 反向代理服务器（hyper）
```

**handle_call 方法列表**：

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `ssh_connect` | `{ host, port, username, password }` | `{ success, message }` | 建立 SSH 连接，自动恢复转发规则 |
| `ssh_disconnect` | -- | `{ success }` | 断开 SSH 连接 |
| `ssh_status` | -- | `SshStatus` | 查询 SSH 连接状态 |
| `kuboard_login` | `{ url, username, password }` | `LoginResult` | 登录 Kuboard（含 MFA 判断） |
| `kuboard_mfa` | `{ passcode }` | `{ success }` | MFA 双因子验证 |
| `kuboard_logout` | -- | `{ success }` | 登出 Kuboard |
| `kuboard_status` | -- | `KuboardStatus` | 查询 Kuboard 登录状态 |
| `list_clusters` | -- | `Vec<String>` | 获取 K8s 集群列表 |
| `list_namespaces` | `{ cluster }` | `Vec<String>` | 获取命名空间列表 |
| `list_pods` | `{ cluster, namespace }` | `Vec<PodInfo>` | 获取 Pod 列表（含容器和端口信息） |
| `forward_pod` | `{ cluster, namespace, pod_name, container_name, container_port }` | `{ rule, proxy_mapping }` | 转发 Pod 端口（SSH 隧道 + 代理注册） |
| `unforward_pod` | `{ rule_id }` | `{ success }` | 取消 Pod 转发 |
| `list_k8s_forwards` | -- | `K8sForwardInfo` | 获取 K8s 转发规则和代理映射 |
| `validate_k8s_forwards` | -- | `{ removed }` | 验证转发规则有效性，移除失效规则 |
| `proxy_start` | `{ port }` | `{ success, message }` | 启动 HTTP 反向代理 |
| `proxy_stop` | -- | `{ success }` | 停止 HTTP 反向代理 |
| `proxy_status` | -- | `ProxyStatus` | 查询代理运行状态 |
| `list_proxy_mappings` | -- | `Vec<ProxyMapping>` | 获取代理映射表 |
| `update_proxy_mapping` | `{ rule_id, domain }` | `ProxyMapping` | 更新代理映射的 Pod 地址 |
| `list_forward_rules` | -- | `Vec<ForwardRule>` | 获取所有转发规则 |
| `add_forward_rule` | `ForwardRule` | `ForwardRule` | 添加手动转发规则 |
| `update_forward_rule` | `ForwardRule` | `ForwardRule` | 更新转发规则 |
| `remove_forward_rule` | `{ id }` | `{ success }` | 删除转发规则 |
| `import_rules` | `{ rules }` | `PluginData` | 批量导入规则 |
| `export_rules` | -- | `Vec<ForwardRule>` | 导出所有规则 |
| `get_config` | -- | `PluginData` | 获取配置（解密凭据） |
| `reset_config` | -- | `{ success }` | 重置所有配置 |

**核心架构**：

```
用户浏览器
  -> HTTP 代理 (本地端口, hyper)
  -> SSH 隧道 (SSH 跳板机, ssh2)
  -> K8s Pod (远程集群)
```

- `SshService`：基于 ssh2 crate 的 SSH 连接管理，每条转发规则一个独立线程，非阻塞 I/O 轮询实现多连接复用
- `KuboardClient`：reqwest HTTP 客户端，cookie jar 管理 Kuboard SSO 会话，支持 SSO 重定向链跟踪和 KuboardToken 捕获
- `HttpProxySvc`：hyper 实现的 HTTP 反向代理，`tokio::select!` 同时处理连接和关闭信号，oneshot channel 实现优雅关闭

**数据存储方式**：
- JSON 文件：`~/.worktools/history/plugins/k8s-forward.json`
- 存储内容：SSH 配置、Kuboard 配置、代理配置、转发规则列表
- 密码字段 AES 加密存储

**依赖的外部库**：

| 库 | 用途 |
|----|------|
| `ssh2` | SSH 连接和端口转发 |
| `hyper` + `hyper-util` + `http-body-util` | HTTP 反向代理服务器 |
| `reqwest` | HTTP 客户端（Kuboard API 调用） |
| `tokio` | 异步运行时（`rt-multi-thread`） |
| `aes` + `sha2` | 密码 AES 加密 |
| `uuid` | 生成规则 ID |
| `once_cell` | 全局静态变量 |

### 前端（React + TypeScript）

**组件结构**：

- `App` -- 主组件，三个标签页切换
- `TabSshForward` -- SSH 端口转发标签页（连接配置 + 转发规则表格）
- `TabK8sForward` -- K8s 服务转发标签页（Kuboard 登录 + 集群/命名空间选择 + Pod 列表 + 已转发列表）
- `TabHttpProxy` -- HTTP 代理标签页（代理启停 + 映射表）

**pluginAPI.call 调用列表**：

| 调用方法 | 组件 | 用途 |
|----------|------|------|
| `ssh_connect` / `ssh_disconnect` / `ssh_status` | TabSshForward | SSH 连接管理 |
| `list_forward_rules` / `add_forward_rule` / `update_forward_rule` / `remove_forward_rule` | TabSshForward | 转发规则 CRUD |
| `import_rules` / `export_rules` | TabSshForward | 规则导入导出 |
| `get_config` | TabSshForward, TabK8sForward | 加载保存的连接配置 |
| `kuboard_login` / `kuboard_mfa` / `kuboard_logout` / `kuboard_status` | TabK8sForward | Kuboard 会话管理 |
| `list_clusters` / `list_namespaces` / `list_pods` | TabK8sForward | K8s 资源浏览 |
| `forward_pod` / `unforward_pod` | TabK8sForward | Pod 转发管理 |
| `list_k8s_forwards` / `validate_k8s_forwards` | TabK8sForward | 转发状态和验证 |
| `update_proxy_mapping` | TabK8sForward, TabHttpProxy | 更新代理映射 |
| `proxy_start` / `proxy_stop` / `proxy_status` | TabHttpProxy | 代理服务器管理 |
| `list_proxy_mappings` | TabHttpProxy | 获取代理映射表 |

**特殊依赖**：
- 无额外第三方前端依赖

## 开发与调试

```bash
# Rust 检查
cargo check -p k8s-forward

# 前端开发
cd plugins/k8s-forward/frontend && npm run dev

# 前端构建
cd plugins/k8s-forward/frontend && npm run build
```

## 已知限制

- SSH 端口自动分配从 10000 开始，最大到 60000 后回绕
- HTTP 代理仅支持 HTTP/1.1，不支持 HTTPS
- Kuboard SSO 登录流程依赖 cookie 重定向，部分环境可能不兼容
- 转发规则验证需要 Kuboard 登录状态，未登录时无法执行
- 每个 SSH 转发占用一个独立线程，大量转发时需注意线程资源
- 插件销毁时按依赖顺序释放资源（先停代理再断 SSH），但异常情况下可能存在端口残留
