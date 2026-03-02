/**
 * 插件组件加载器
 *
 * 实现动态导入,解耦 App.tsx 和具体插件组件的依赖
 * 同时保留 Solid.js 组件的完整功能和开发体验
 */

import { Component, Setter } from "solid-js";
import PluginView from "../components/PluginView";

interface PluginComponentProps {
  pluginId: string;
  setSelectedPlugin: Setter<string | null>;
}

export type PluginComponentType = Component<PluginComponentProps>;

/**
 * 插件组件映射表
 *
 * 维护插件 ID 到组件的映射关系
 * 新增插件时只需在此处添加映射,无需修改 App.tsx
 */
const PLUGIN_COMPONENT_MAP: Record<
  string,
  () => Promise<{ default: PluginComponentType }>
> = {
  // 内置插件 - 使用 Solid.js 组件以获得最佳用户体验
  "password-manager": () =>
    import("../components/PasswordManager").then((m) => ({
      default: m.default,
    })),
  auth: () =>
    import("../components/AuthPlugin").then((m) => ({
      default: m.default,
    })),

  // 外部插件 - 使用 PluginView 加载 HTML
  // 未来可以通过插件包动态注册
};

/**
 * 动态加载插件组件
 *
 * @param pluginId 插件 ID
 * @returns 插件组件或回退到 PluginView
 */
export async function loadPluginComponent(
  pluginId: string,
): Promise<PluginComponentType> {
  // 检查是否有专门的 Solid.js 组件
  const componentLoader = PLUGIN_COMPONENT_MAP[pluginId];

  if (componentLoader) {
    try {
      console.log(`[PluginLoader] 加载 Solid.js 组件: ${pluginId}`);
      const module = await componentLoader();
      return module.default;
    } catch (error) {
      console.error(`[PluginLoader] 加载组件失败: ${pluginId}`, error);
      // 加载失败时回退到 PluginView
      return PluginView;
    }
  }

  // 没有专门组件的插件,使用通用的 PluginView
  console.log(`[PluginLoader] 使用通用 PluginView: ${pluginId}`);
  return PluginView;
}

/**
 * 检查插件是否有专门的 Solid.js 组件
 *
 * @param pluginId 插件 ID
 * @returns 是否有专门组件
 */
export function hasNativeComponent(pluginId: string): boolean {
  return pluginId in PLUGIN_COMPONENT_MAP;
}

/**
 * 获取所有已注册的插件 ID
 *
 * @returns 插件 ID 数组
 */
export function getRegisteredPluginIds(): string[] {
  return Object.keys(PLUGIN_COMPONENT_MAP);
}
