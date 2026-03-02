import { createSignal, onMount, Show, Setter, createEffect } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { createPluginBridge } from "../utils/pluginBridge";
import "./PluginView.css";

interface PluginViewProps {
  pluginId: string;
  setSelectedPlugin: Setter<string | null>;
}

export default (props: PluginViewProps) => {
  const [html, setHtml] = createSignal<string>("");
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string>("");

  // 使用 createEffect 代替 onMount,以便在 pluginId 变化时重新加载
  createEffect(async () => {
    const pluginId = props.pluginId;
    if (!pluginId) return;

    console.log("[PluginView] 开始加载插件:", pluginId);
    try {
      setLoading(true);
      setError("");

      // 首先尝试获取插件前端资源
      try {
        const indexHtml = await invoke<string>("read_plugin_asset", {
          pluginId: pluginId,
          assetPath: "index.html",
        });

        console.log("[PluginView] 获取到前端资源 HTML,长度:", indexHtml.length);

        // 读取并内联 JS
        let processedHtml = indexHtml;

        try {
          // 读取 CSS
          const styles = await invoke<string>("read_plugin_asset", {
            pluginId: pluginId,
            assetPath: "styles.css",
          });

          // 读取 JS
          const script = await invoke<string>("read_plugin_asset", {
            pluginId: pluginId,
            assetPath: "main.js",
          });

          // 移除外部链接,因为我们会在主文档中渲染
          processedHtml = processedHtml.replace(
            /<link rel="stylesheet" href="styles.css">/,
            "",
          );

          // 将 CSS 注入到主文档的 head 中(使用一个特殊的 style 标签)
          const styleId = `plugin-styles-${pluginId}`;
          // 先移除旧的样式
          const oldStyle = document.getElementById(styleId);
          if (oldStyle) oldStyle.remove();

          // 添加新的样式
          const styleEl = document.createElement("style");
          styleEl.id = styleId;
          styleEl.textContent = styles;
          document.head.appendChild(styleEl);

          // 将 JS 内联到 HTML 中
          processedHtml = processedHtml.replace(
            /<script src="main.js"><\/script>/,
            `<script>${script}<\/script>`,
          );

          console.log("[PluginView] CSS 已注入到主文档,JS 已内联");
        } catch (err) {
          console.warn("[PluginView] 无法加载 CSS/JS:", err);
        }

        // 直接使用 innerHTML 渲染,不使用 iframe
        setHtml(processedHtml);
        setLoading(false);

        // 等待 DOM 更新后创建插件桥
        setTimeout(async () => {
          const bridge = createPluginBridge(pluginId);
          bridge.exposeToWindow();
          console.log("[PluginView] 插件桥已暴露到 window");

          // 对于 password-manager,自动加载数据
          if (pluginId === "password-manager") {
            try {
              const result = await bridge.call("list_passwords");
              console.log("[PluginView] 密码列表:", result);

              // 调用页面上的初始化函数(如果存在)
              if ((window as any).initPasswordManager) {
                (window as any).initPasswordManager(result.entries || []);
              }
            } catch (err) {
              console.error("[PluginView] 加载初始数据失败:", err);
            }
          }
        }, 100);

        console.log("[PluginView] 使用 innerHTML 模式加载插件");
        return;
      } catch (err) {
        console.log("[PluginView] 未找到前端资源,使用传统 HTML 模式:", err);
        // 如果没有前端资源,回退到传统的 HTML 模式
      }

      // 传统模式:从插件获取 HTML
      const viewHtml = await invoke<string>("get_plugin_view", {
        pluginId: pluginId,
      });

      console.log(
        "[PluginView] 获取到 HTML 内容:",
        viewHtml.substring(0, 100) + "...",
      );
      setHtml(viewHtml);

      // 创建插件桥并暴露到 window
      const bridge = createPluginBridge(pluginId);
      bridge.exposeToWindow();
      console.log("[PluginView] 插件桥已暴露到 window");

      // 等待DOM更新后自动加载初始数据
      setTimeout(async () => {
        try {
          // 对于 password-manager,自动加载数据
          if (pluginId === "password-manager") {
            console.log("[PluginView] 开始加载密码列表...");
            const result = await bridge.call("list_passwords");
            console.log("[PluginView] 密码列表:", result);

            // 更新DOM显示数据
            const listEl = document.getElementById("password-list");
            console.log("[PluginView] 找到 password-list 元素:", listEl);

            if (listEl && result.entries) {
              if (result.entries.length === 0) {
                listEl.innerHTML = "<p>暂无密码条目</p>";
              } else {
                listEl.innerHTML = result.entries
                  .map(
                    (entry: any) => `
                  <div style="padding: 10px; border: 1px solid #ddd; margin-bottom: 5px; border-radius: 4px;">
                    <strong>${entry.service}</strong> - ${entry.username}
                  </div>
                `,
                  )
                  .join("");
              }
            }
          }
        } catch (err) {
          console.error("[PluginView] 加载初始数据失败:", err);
        }
      }, 100);

      setLoading(false);
      console.log("[PluginView] 插件加载完成");
    } catch (err) {
      console.error("[PluginView] 加载插件失败:", err);
      setError(err as string);
      setLoading(false);
    }
  });

  return (
    <div class="plugin-view">
      <Show when={loading()}>
        <div class="loading">加载中...</div>
      </Show>

      <Show when={error()}>
        <div class="error">加载失败: {error()}</div>
      </Show>

      <Show when={!loading() && !error() && html()}>
        <div innerHTML={html()} class="plugin-content" />
      </Show>
    </div>
  );
};
