export interface KeyInfo {
  key: string;
  type: string;
  ttl: number;
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
