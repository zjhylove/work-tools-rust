# k8s-forward SSH 自动重连设计

## 背景

k8s-forward 插件通过 SSH 隧道实现端口转发，当前 SSH 连接是一次性建立的，没有心跳检测和自动重连。当 SSH session 因网络抖动、服务端超时等原因断连时，所有转发失效，用户需要手动断开再重连。

## 目标

在 SSH 连接断开时自动重连，并恢复所有活跃的转发规则，减少用户手动干预。

## 方案

**后台心跳检测线程**：在 `SshService` 中新增独立心跳线程，定期检查 SSH session 存活状态，检测到断连后自动触发重连。

## 设计

### 1. 心跳检测

在 `SshService` 中新增心跳守护线程，SSH 连接处于 `connected` 状态时运行：

- **检测间隔**：每 15 秒
- **检测方式**：调用 `ssh2::Session::is_connected()`
- **断连判定**：连续 2 次检测失败才判定为断连（避免单次网络抖动误判）
- **线程生命周期**：`ssh_connect` 时启动，`disconnect` 时通过 stop flag 停止；重连成功后同一线程持续监控，无需重启

### 2. 重连流程

检测到断连后：

1. **保存连接参数**：`ssh_connect` 成功后将 host、port、username、password 缓存在 `SshService.last_connect_params` 字段
2. **重连前清理**：停止所有转发线程，清理 `forwards`、`threads`、`stop_flags`，置空 `session`
3. **重连执行**：使用保存的参数重新建立 TCP 连接 → SSH 握手 → 认证
4. **转发恢复**：重连成功后从持久化的 `forward_rules` 中恢复所有之前活跃的转发规则，为每条规则重新创建转发线程
5. **指数退避**：初始间隔 2s，每次失败翻倍，最大间隔 60s
6. **最大重试次数**：10 次，超过后停止重试，状态置为 `disconnected`
7. **手动取消**：重连过程中用户可调用 `ssh_disconnect` 取消
8. **手动重连**：重试耗尽后用户可通过 `ssh_connect`（新参数）或 `ssh_reconnect`（已保存参数）重新发起连接

### 3. 状态管理

连接状态在现有 `connected` / `disconnected` 基础上新增 `reconnecting`：

**`ssh_status` 返回值**：
- `status: "connected"` — 正常连接
- `status: "reconnecting"` — 自动重连中，附带 `retry_count`、`max_retries`、`next_retry_at`
- `status: "disconnected"` — 已断开，如果 `reconnect_failed: true` 表示已耗尽重试次数

**前端展示**（`TabSshForward.tsx`、`TabK8sForward.tsx`）：
- `reconnecting` 时显示"重连中 (第 N/10 次)..."
- 重连成功后自动恢复为"已连接"，转发列表状态同步更新
- 重试耗尽后显示"连接已断开，请手动重连"并提供重连按钮
- 前端通过现有的定期刷新逻辑读取 `ssh_status`，无需新增额外轮询

### 4. handle_call 方法变更

| 方法 | 变更类型 | 说明 |
|------|---------|------|
| `ssh_connect` | 变更 | 连接成功后保存连接参数，启动心跳线程 |
| `ssh_disconnect` | 变更 | 确保同时停止心跳线程和正在进行的重连 |
| `ssh_status` | 变更 | 返回值增加 `status` 字段及重连元信息 |
| `ssh_reconnect` | 新增 | 手动触发重连，使用保存的参数，重置重试计数器 |

### 5. 涉及文件

| 文件 | 变更 |
|------|------|
| `plugins/k8s-forward/src/ssh_service.rs` | 心跳线程、重连逻辑、连接参数缓存、状态管理 |
| `plugins/k8s-forward/src/lib.rs` | 新增 `ssh_reconnect` handle_call 分支，`ssh_status` 返回值变更 |
| `plugins/k8s-forward/src/models.rs` | 新增连接状态枚举、重连元信息结构体 |
| `plugins/k8s-forward/frontend/src/types.ts` | 状态类型更新 |
| `plugins/k8s-forward/frontend/src/TabSshForward.tsx` | 重连中/失败状态展示 |
| `plugins/k8s-forward/frontend/src/TabK8sForward.tsx` | 重连中/失败状态展示 |
