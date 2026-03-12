import { invoke } from "@tauri-apps/api/core";

/**
 * 插件通信桥
 *
 * 负责前端与插件后端的方法调用通信
 */
export class PluginBridge {
  constructor(private pluginId: string) {}

  /**
   * 调用插件方法
   * @param method 方法名
   * @param params 参数
   * @returns 返回值
   */
  async call(method: string, params: any = {}): Promise<any> {
    try {
      return await invoke("call_plugin_method", {
        pluginId: this.pluginId,
        method,
        params,
      });
    } catch (error) {
      console.error(`[PluginBridge] 调用失败 ${this.pluginId}.${method}:`, error);
      throw error;
    }
  }

  /**
   * 将插件 API 暴露到全局 window 对象
   * 用于插件 HTML 内联脚本调用
   */
  exposeToWindow() {
    if (typeof window === "undefined") return;

    (window as any).pluginAPI = {
      call: this.call.bind(this),
    };
  }

  /**
   * 获取插件 API 对象
   * 用于手动暴露到 iframe 的 window 对象
   */
  getAPI() {
    return {
      call: this.call.bind(this),
    };
  }
}

/**
 * 创建插件桥实例
 * @param pluginId 插件 ID
 * @returns PluginBridge 实例
 */
export function createPluginBridge(pluginId: string): PluginBridge {
  return new PluginBridge(pluginId);
}
