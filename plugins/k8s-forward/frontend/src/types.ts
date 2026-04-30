declare global {
  interface Window {
    pluginAPI: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>;
    };
  }
}

export interface ForwardRule {
  id: string;
  name: string;
  local_host: string;
  local_port: number;
  remote_host: string;
  remote_port: number;
  rule_type: "Manual" | "K8s";
  cluster?: string;
  namespace?: string;
  pod_name?: string;
  container_name?: string;
}

export interface ProxyMapping {
  domain: string;
  target: string;
  rule_id: string;
  editable: boolean;
}

export interface SshStatus {
  connected: boolean;
  host?: string;
  port?: number;
}

export interface KuboardStatus {
  logged_in: boolean;
  url?: string;
  username?: string;
}

export interface ProxyStatus {
  running: boolean;
  port: number;
  mapping_count: number;
}

export interface PodInfo {
  name: string;
  ip: string;
  status: string;
  containers: ContainerInfo[];
}

export interface ContainerInfo {
  name: string;
  ports: ContainerPort[];
}

export interface ContainerPort {
  name?: string;
  container_port: number;
  protocol: string;
}

export interface LoginResult {
  success: boolean;
  mfa_required?: boolean;
  message?: string;
}

export interface K8sForwardInfo {
  rules: ForwardRule[];
  mappings: ProxyMapping[];
}
