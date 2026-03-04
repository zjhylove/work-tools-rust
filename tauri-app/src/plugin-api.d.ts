/**
 * 插件 API 类型定义
 * 这个接口定义了插件前端代码可用的 window.pluginAPI 对象
 */
interface PluginAPI {
  /**
   * 调用插件方法
   * @param pluginId - 插件 ID (例如: "password-manager")
   * @param method - 方法名 (例如: "list_passwords")
   * @param params - 方法参数对象
   * @returns 方法执行结果
   */
  call: (
    pluginId: string,
    method: string,
    params: Record<string, unknown>
  ) => Promise<unknown>;

  /**
   * 获取插件配置
   * @param pluginId - 插件 ID
   * @returns 插件配置对象
   */
  get_plugin_config: (pluginId: string) => Promise<Record<string, unknown>>;

  /**
   * 保存插件配置
   * @param pluginId - 插件 ID
   * @param config - 配置对象
   */
  set_plugin_config: (
    pluginId: string,
    config: Record<string, unknown>
  ) => Promise<void>;

  /**
   * 打开外部链接
   * @param url - 要打开的 URL
   */
  open_url: (url: string) => Promise<void>;
}

/**
 * 扩展 Window 接口,添加 pluginAPI 属性
 */
declare global {
  interface Window {
    pluginAPI: PluginAPI;
  }
}

// 确保这个文件被视为模块
export {};
