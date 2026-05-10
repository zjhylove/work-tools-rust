export const COLORS = ['#ef4444', '#f59e0b', '#10b981', '#3b82f6', '#8b5cf6', '#ec4899', '#06b6d4', '#6b7280'];

export interface KeyInfo {
  key: string;
  type: string;
  ttl: number;
}

export interface SshInfo {
  host: string;
  port: number;
  username: string;
  auth_type: 'password' | 'key';
  has_auth: boolean;
  timeout_secs: number;
}

export interface ClusterInfo {
  seed_nodes: string;
}

export interface SavedConnection {
  id: string;
  name: string;
  color: string | null;
  host: string;
  port: number;
  db: number;
  has_password: boolean;
  has_ssh: boolean;
  has_cluster: boolean;
  ssh?: SshInfo | null;
  cluster?: ClusterInfo | null;
}

export interface ConnectionForm {
  name: string;
  color: string | null;
  host: string;
  port: number;
  db: number;
  password: string;
  ssh: SshForm | null;
  cluster: ClusterForm | null;
}

export interface SshForm {
  host: string;
  port: number;
  username: string;
  authType: 'password' | 'key';
  password: string;
  keyPath: string;
  keyPassphrase: string;
  timeoutSecs: number;
}

export interface ClusterForm {
  seedNodes: string;
}

export interface TreeNode {
  name: string;
  fullKey: string | null;
  prefix: string;
  keyInfo?: KeyInfo;
  children: TreeNode[];
}

export type AppView = 'connect' | 'workspace' | 'manager';

export interface ConnectionInfo {
  connected: boolean;
  id?: string;
  name?: string;
  host?: string;
  port?: number;
  db?: number;
  color?: string | null;
}
