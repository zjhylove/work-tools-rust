# Multi-SSH Connections for K8s-Forward

**Date**: 2026-05-27
**Status**: Draft

## Overview

Extend k8s-forward plugin to support multiple simultaneous SSH connections. Each port forward rule is associated with a specific SSH connection. When creating a forward, users select which SSH connection to use.

## Requirements

1. Support 2-3 simultaneous SSH connections (no hard upper limit)
2. Each forward rule belongs to exactly one SSH connection
3. K8s Pod forwarding also requires SSH connection selection
4. HTTP proxy is shared across all connections, aggregating mappings from all active forwards
5. Automatic migration from single-SSH to multi-SSH data format

## Data Model Changes (models.rs)

### New: `SshConnection`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConnection {
    pub id: String,
    pub name: String,
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub username: String,
    pub password: String, // encrypted
}
```

### Modified: `PluginData`

- Replace `ssh: Option<SshConfig>` with `ssh_connections: Vec<SshConnection>`
- Keep backward-compatible deserialization: if old `ssh` field exists and `ssh_connections` is empty, auto-migrate

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginData {
    #[serde(default)]
    pub ssh_connections: Vec<SshConnection>,
    #[serde(default)]
    pub kuboard: Option<KuboardConfig>,
    #[serde(default)]
    pub proxy: ProxyConfig,
    #[serde(default)]
    pub forward_rules: Vec<ForwardRule>,
}
```

### Modified: `ForwardRule`

Add `ssh_connection_id` field:

```rust
pub struct ForwardRule {
    // ... existing fields ...
    #[serde(default)]
    pub ssh_connection_id: String,
}
```

### Migration strategy

In `K8sForwardPlugin::load_data()`, after deserializing `PluginData`, check and migrate:

1. Read old JSON with both `ssh` and `ssh_connections` fields. Keep `SshConfig` struct with `#[serde(default)]` for backward compat read:
   ```rust
   pub struct PluginData {
       #[serde(default)]
       pub ssh_connections: Vec<SshConnection>,
       #[serde(default)]
       pub ssh: Option<SshConfig>,  // legacy, used only for migration
       // ...
   }
   ```
2. In `load_data()`: if `ssh_connections` is empty and `ssh` is Some, create a `SshConnection` from the old `SshConfig` with a generated UUID and `name = host`, then set all `forward_rules[].ssh_connection_id` to this new ID
3. After migration, call `save_data()` to persist the new format
4. On serialize, the `ssh` field is skipped (`#[serde(skip_serializing)]`) so the new format is clean

## Backend Architecture Changes (lib.rs)

### K8sForwardPlugin struct

```rust
pub struct K8sForwardPlugin {
    storage: PluginStorage,
    encryptor: PasswordEncryptor,
    runtime: Runtime,
    ssh_connections: Mutex<HashMap<String, SshService>>,  // was: Mutex<SshService>
    proxy: Mutex<Option<HttpProxySvc>>,
    kuboard: Mutex<Option<KuboardClient>>,
}
```

### New handle_call methods

| Method | Params | Description |
|---|---|---|
| `ssh_add_connection` | `{name, host, port, username, password}` | Add SSH connection config (does not connect) |
| `ssh_update_connection` | `{id, name?, host?, port?, username?, password?}` | Update connection config |
| `ssh_remove_connection` | `{id}` | Disconnect if active, stop all associated forwards, remove connection config AND its associated forward_rules from PluginData |
| `ssh_list_connections` | - | Return all connections with status info |

### Modified handle_call methods

| Method | Change |
|---|---|
| `ssh_connect` | Accept `connection_id` param, operate on specific SshService instance |
| `ssh_disconnect` | Accept `connection_id` param, only disconnect that connection |
| `ssh_reconnect` | Accept `connection_id` param |
| `ssh_status` | Accept `connection_id` param, return status for that connection |
| `add_forward_rule` | Rule must include `ssh_connection_id` |
| `forward_pod` | Accept `ssh_connection_id`, create forward on specific connection |
| `update_forward_rule` | May include `ssh_connection_id` update |
| `list_forward_rules` | Return rules with `ssh_connection_id` field |
| `list_k8s_forwards` | Return rules with `ssh_connection_id` field |
| `import_rules` | Handle `ssh_connection_id` in imported rules |

### Key logic changes

**restore_forwards**: On SSH connect, only restore forward rules matching `ssh_connection_id`.

**ssh_connect**: No longer disconnects other connections. Finds or creates the `SshService` for the given `connection_id`.

**ssh_disconnect**: Only stops the specified connection's forwards and heartbeat/reconnect threads.

**HTTP proxy aggregation**: On proxy start, collect mappings from all active SSH connections' forward rules.

### Heartbeat and reconnect

Each `SshService` instance maintains its own heartbeat thread and reconnect logic independently.

## Frontend Changes

### Types (types.ts)

```typescript
export interface SshConnection {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  password: string;
}

export interface ForwardRule {
  // ... existing fields ...
  ssh_connection_id: string;
}

export interface SshStatus {
  connected: boolean;
  host?: string;
  port?: number;
  status: SshConnectionState;
  reconnect_info?: ReconnectInfo;
  connection_id: string;  // new
  connection_name: string;  // new
}
```

### TabSshForward redesign

**Connection management area (top)**:
- Connection list: each row shows `name | host:port | status dot | connect/disconnect button | edit/delete button`
- "+ Add SSH" button in top-right, opens modal for name/host/port/username/password
- Status dots: green (Connected), yellow blink (Reconnecting), gray (Disconnected)

**Forward rules area (bottom)**:
- Dropdown "SSH Connection:" to filter rules by connection
- "All" option shows all rules with an extra column showing connection name
- Adding a rule auto-binds to the currently selected connection
- Import/export: rules include `ssh_connection_id`

### TabK8sForward changes

- Add dropdown "Via SSH:" above the Pod list
- Dropdown shows all configured SSH connections, only connected ones are selectable
- When forwarding a Pod, uses the selected SSH connection
- Active forwards list shows connection name column

### TabHttpProxy changes

- Minimal change: proxy aggregates mappings from all active SSH connections
- No SSH selection needed in this tab

## Error Handling

- Connecting to a connection that is already connected: return error "Already connected"
- Forwarding on a disconnected SSH: return error with connection name, suggest connecting first
- Removing a connection that has active forwards: auto-stop forwards, remove all associated forward_rules from PluginData, and show toast with count of removed rules
- Duplicate connection names: allowed (disambiguated by host:port display)

## Testing Strategy

- Unit tests for data migration (old single-SSH format → new multi-SSH format)
- Unit tests for `SshService` multi-instance independence
- Integration test: add 2 connections, connect both, verify forwards work independently
- Frontend: verify connection list UI, rule filtering, K8s SSH selection
