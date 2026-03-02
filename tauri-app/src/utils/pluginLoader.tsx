import React from "react";

/**
 * 创建支持懒加载的插件组件
 *
 * @param componentLoader 组件加载函数
 * @returns React 懒加载组件
 */
export function createLazyPluginComponent(
  componentLoader: () => Promise<{ default: any }>,
): React.LazyExoticComponent<React.ComponentType<any>> {
  // React.lazy 需要一个返回 Promise 的函数
  // componentLoader 本身就是这样的函数，直接使用
  return React.lazy(componentLoader);
}
