/**
 * Plugin API 类型定义
 * 由主程序注入到 window 对象
 */

export interface PluginAPI {
  call: (pluginId: string, method: string, params: Record<string, unknown>) => Promise<unknown>;
  get_plugin_config: (pluginId: string) => Promise<Record<string, unknown>>;
  set_plugin_config: (pluginId: string, config: Record<string, unknown>) => Promise<void>;
}

declare global {
  interface Window {
    pluginAPI?: PluginAPI;
  }
}

export {};
