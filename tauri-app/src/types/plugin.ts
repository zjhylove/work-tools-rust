/**
 * 插件相关类型定义
 * 统一管理所有插件相关的类型,避免重复定义
 */

/**
 * 插件清单信息 (从 manifest.json 读取)
 */
export interface PluginManifest {
  id: string;
  name: string;
  description: string;
  version: string;
  icon?: string;
  author?: string;
  homepage?: string;
}

/**
 * 插件信息 (显示在侧边栏)
 */
export interface PluginInfo {
  id: string;
  name: string;
  description: string;
  version: string;
  icon: string;
}

/**
 * 插件商店中的插件信息 (扩展自 PluginManifest)
 */
export interface StorePluginInfo extends PluginManifest {
  installed: boolean;
}

/**
 * 已安装的插件详细信息
 */
export interface InstalledPlugin extends PluginManifest {
  installed_at: string;
  enabled: boolean;
  assets_path: string;
  library_path: string;
}

/**
 * 插件包信息 (用于插件市场)
 */
export interface PluginPackage {
  manifest: PluginManifest;
  filePath: string;
  size?: number;
}

/**
 * 插件操作结果
 */
export interface PluginOperationResult {
  success: boolean;
  message: string;
  pluginId?: string;
}
