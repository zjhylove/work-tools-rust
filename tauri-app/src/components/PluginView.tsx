import { createSignal, onMount, Show, Setter, createEffect, on } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { createPluginBridge } from "../utils/pluginBridge";
import "./PluginView.css";

interface PluginViewProps {
  pluginId: string;
  setSelectedPlugin: Setter<string | null>;
}

export default (props: PluginViewProps) => {
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string>("");
  let iframeRef: HTMLIFrameElement | undefined;

  // 使用 createEffect 代替 onMount,以便在 pluginId 变化时重新加载
  createEffect(
    on(
      () => props.pluginId,
      async (pluginId) => {
        if (!pluginId) return;

        console.log("[PluginView] 开始加载插件:", pluginId);
        try {
          setLoading(true);
          setError("");

          // 等待 iframe 准备就绪
          await new Promise((resolve) => setTimeout(resolve, 100));

          const iframe = iframeRef;
          if (!iframe) {
            throw new Error("iframe 元素未找到");
          }

          // 获取 iframe 的 document
          const iframeDoc =
            iframe.contentDocument || iframe.contentWindow?.document;
          if (!iframeDoc) {
            throw new Error("无法访问 iframe document");
          }

          // 首先尝试获取插件前端资源
          try {
            const indexHtml = await invoke<string>("read_plugin_asset", {
              pluginId: pluginId,
              assetPath: "index.html",
            });

            console.log(
              "[PluginView] 获取到前端资源 HTML,长度:",
              indexHtml.length,
            );

            // 读取并内联 JS/CSS
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

              // 将 CSS 注入到 iframe 的 head 中
              const styleEl = iframeDoc.createElement("style");
              styleEl.textContent = styles;
              iframeDoc.head.appendChild(styleEl);

              // 将 JS 内联到 HTML 中
              processedHtml = processedHtml.replace(
                /<script src="main.js"><\/script>/,
                `<script>${script}<\/script>`,
              );

              console.log(
                "[PluginView] CSS 已注入到 iframe,JS 已内联",
              );
            } catch (err) {
              console.warn("[PluginView] 无法加载 CSS/JS:", err);
            }

            // 写入 iframe 的 HTML
            iframeDoc.open();
            iframeDoc.write(processedHtml);
            iframeDoc.close();

            console.log("[PluginView] 使用 iframe 模式加载插件");

            // 等待 iframe 中的脚本执行
            setTimeout(async () => {
              // 创建插件桥并暴露到 iframe 的 window
              const bridge = createPluginBridge(pluginId);
              if (iframe.contentWindow) {
                (iframe.contentWindow as any).pluginAPI = bridge.getAPI();
                console.log("[PluginView] 插件桥已暴露到 iframe");
              }
            }, 200);

            setLoading(false);
            return;
          } catch (err) {
            console.log(
              "[PluginView] 未找到前端资源,使用传统 HTML 模式:",
              err,
            );
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

          // 写入 iframe 的 HTML
          iframeDoc.open();
          iframeDoc.write(viewHtml);
          iframeDoc.close();

          // 创建插件桥并暴露到 iframe 的 window
          const bridge = createPluginBridge(pluginId);
          if (iframe.contentWindow) {
            (iframe.contentWindow as any).pluginAPI = bridge.getAPI();
            console.log("[PluginView] 插件桥已暴露到 iframe");
          }

          setLoading(false);
          console.log("[PluginView] 插件加载完成");
        } catch (err) {
          console.error("[PluginView] 加载插件失败:", err);
          setError(err as string);
          setLoading(false);
        }
      },
    ),
  );

  return (
    <div class="plugin-view">
      <Show when={loading()}>
        <div class="loading">加载中...</div>
      </Show>

      <Show when={error()}>
        <div class="error">加载失败: {error()}</div>
      </Show>

      <Show when={!loading() && !error()}>
        <iframe
          ref={iframeRef}
          class="plugin-content"
          style={{
            width: "100%",
            height: "100%",
            border: "none",
            "background-color": "var(--content-area-bg, #ffffff)",
          }}
          sandbox="allow-scripts allow-same-origin allow-forms allow-modals"
        />
      </Show>
    </div>
  );
};
