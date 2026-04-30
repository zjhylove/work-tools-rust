interface PluginAPI {
  call(pluginId: string, method: string, params: Record<string, unknown>): Promise<unknown>;
  get_plugin_config(pluginId: string): Promise<unknown>;
  set_plugin_config(pluginId: string, config: unknown): Promise<void>;
  open_url(url: string): Promise<void>;
  open_folder_dialog(title?: string): Promise<string | null>;
  write_file(path: string, content: string): Promise<void>;
}

interface Window {
  pluginAPI?: PluginAPI;
}
