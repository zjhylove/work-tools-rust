export interface PluginAPI {
  call: (pluginId: string, method: string, params: Record<string, unknown>) => Promise<unknown>;
  get_plugin_config: (pluginId: string) => Promise<Record<string, unknown>>;
  set_plugin_config: (pluginId: string, config: Record<string, unknown>) => Promise<void>;
  open_folder_dialog: (title: string) => Promise<string | null>;
  open_file_dialog: (title: string) => Promise<string | null>;
  write_file: (path: string, content: string) => Promise<void>;
}

declare global {
  interface Window {
    pluginAPI?: PluginAPI;
  }
}

export {};
