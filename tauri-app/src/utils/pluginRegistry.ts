/**
 * 插件注册表
 *
 * 维护所有插件的元数据和组件加载器
 * 新增插件时在此注册即可
 */
export interface PluginRegistration {
  id: string;
  name: string;
  description: string;
  version: string;
  icon: string;
  componentLoader: () => Promise<{ default: any }>;
}

const PLUGIN_REGISTRY: Record<string, PluginRegistration> = {
  "password-manager": {
    id: "password-manager",
    name: "密码管理器",
    description: "本地安全存储和管理密码",
    version: "1.0.0",
    icon: "🔐",
    componentLoader: () => import("../components/PasswordManagerReact"),
  },
  auth: {
    id: "auth",
    name: "双因素验证",
    description: "TOTP 双因素认证",
    version: "1.0.0",
    icon: "🔐",
    componentLoader: () => import("../components/AuthPluginReact"),
  },
};

/**
 * 注册新插件
 *
 * @param plugin 插件注册信息
 */
export function registerPlugin(plugin: PluginRegistration): void {
  PLUGIN_REGISTRY[plugin.id] = plugin;
}

/**
 * 获取插件组件加载器
 *
 * @param pluginId 插件 ID
 * @returns 组件加载函数或 null
 */
export function getPluginLoader(
  pluginId: string,
): PluginRegistration["componentLoader"] | null {
  return PLUGIN_REGISTRY[pluginId]?.componentLoader || null;
}

/**
 * 获取所有已注册插件
 *
 * @returns 插件注册信息数组
 */
export function getAllPlugins(): PluginRegistration[] {
  return Object.values(PLUGIN_REGISTRY);
}

/**
 * 检查插件是否已注册
 *
 * @param pluginId 插件 ID
 * @returns 是否已注册
 */
export function isPluginRegistered(pluginId: string): boolean {
  return pluginId in PLUGIN_REGISTRY;
}
