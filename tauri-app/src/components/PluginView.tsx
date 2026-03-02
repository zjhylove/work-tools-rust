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
  const [assetsUrl, setAssetsUrl] = createSignal<string>("");
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string>("");
  const [useIframe, setUseIframe] = createSignal(false);

  // 使用 createEffect 代替 onMount,以便在 pluginId 变化时重新加载
  createEffect(async () => {
    const pluginId = props.pluginId;
    if (!pluginId) return;

    console.log("[PluginView] 开始加载插件:", pluginId);
    try {
      setLoading(true);
      setError("");
      setUseIframe(false);

      // 首先尝试获取插件前端资源
      try {
        const indexHtml = await invoke<string>("read_plugin_asset", {
          pluginId: pluginId,
          assetPath: "index.html",
        });

        console.log("[PluginView] 获取到前端资源 HTML,长度:", indexHtml.length);

        // 设置 iframe 内容使用 srcdoc
        setHtml(indexHtml);
        setUseIframe(true);
        setLoading(false);
        console.log("[PluginView] 使用 iframe srcdoc 模式加载插件");
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

      <Show when={!loading() && !error()}>
        <Show when={useIframe() && html()}>
          <iframe
            srcdoc={html()}
            class="plugin-iframe"
            sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-modals"
          />
        </Show>

        <Show when={!useIframe() && html()}>
          <div innerHTML={html()} class="plugin-content" />
        </Show>
      </Show>
    </div>
  );
};
