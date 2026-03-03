import React, { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface PluginPlaceholderProps {
  pluginId: string;
  setSelectedPlugin: (pluginId: string | null) => void;
}

export default function PluginPlaceholder({
  pluginId,
  setSelectedPlugin,
}: PluginPlaceholderProps) {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>("");
  const [htmlContent, setHtmlContent] = useState<string>("");
  const iframeRef = useRef<HTMLIFrameElement>(null);

  useEffect(() => {
    const loadPlugin = async () => {
      if (!pluginId) return;

      console.log("[PluginPlaceholder] 开始加载插件:", pluginId);
      try {
        setLoading(true);
        setError("");

        // 读取插件的 index.html 内容
        try {
          let html: string;
          let mainJs: string;
          let styles: string;

          try {
            html = await invoke<string>("read_plugin_asset", {
              pluginId: pluginId,
              assetPath: "index.html",
            });
          } catch (e) {
            throw new Error(`读取 index.html 失败: ${e}`);
          }

          try {
            mainJs = await invoke<string>("read_plugin_asset", {
              pluginId: pluginId,
              assetPath: "main.js",
            });
          } catch (e) {
            throw new Error(`读取 main.js 失败: ${e}`);
          }

          try {
            styles = await invoke<string>("read_plugin_asset", {
              pluginId: pluginId,
              assetPath: "styles.css",
            });
          } catch (e) {
            throw new Error(`读取 styles.css 失败: ${e}`);
          }

          console.log("[PluginPlaceholder] 插件资源读取成功");

          // 获取父页面的 CSS 变量
          const cssVars = `
            :root {
              --bg-primary: #ffffff;
              --bg-secondary: #fafafa;
              --bg-tertiary: #f5f5f5;
              --hover-bg: #f0f0f0;
              --text-primary: #000000;
              --text-secondary: #666666;
              --text-tertiary: #999999;
              --border-color: #e5e5e5;
              --border-light: #f0f0f0;
              --accent: #007aff;
              --accent-hover: #0063d1;
              --accent-light: #e5f1ff;
              --success-color: #34c759;
              --warning-color: #ff9500;
              --error-color: #ff3b30;
              --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.04);
              --shadow-md: 0 2px 8px rgba(0, 0, 0, 0.06);
              --shadow-lg: 0 4px 16px rgba(0, 0, 0, 0.08);
              --radius-sm: 4px;
              --radius-md: 8px;
              --radius-lg: 12px;
              --radius-xl: 16px;
            }
            * {
              box-sizing: border-box;
            }
            body {
              margin: 0;
              padding: 0;
              font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
            }
          `;

          // 直接在 HTML 中内联 CSS 和 JavaScript
          // 使用 split + join 方法只替换第一个匹配项
          const parts = html.split(
            '<script type="module" crossorigin src="./main.js"></script>',
          );
          const fullHtml =
            parts[0] +
            `<style>${cssVars}${styles}</style><script type="module">${mainJs}</script>` +
            parts
              .slice(1)
              .join(
                '<script type="module" crossorigin src="./main.js"></script>',
              )
              .split('<link rel="stylesheet" crossorigin href="./styles.css">')
              .join("");

          console.log("[PluginPlaceholder] HTML 长度:", fullHtml.length);
          console.log(
            "[PluginPlaceholder] HTML 包含 <script>:",
            fullHtml.includes('<script type="module">'),
          );
          console.log(
            "[PluginPlaceholder] HTML 包含 <style>:",
            fullHtml.includes("<style>"),
          );
          console.log(
            "[PluginPlaceholder] HTML 前 200 字符:",
            fullHtml.substring(0, 200),
          );

          setHtmlContent(fullHtml);
        } catch (err) {
          console.error("[PluginPlaceholder] 读取插件资源失败:", err);
          if (err instanceof Error) {
            setError(
              `插件 "${pluginId}" 未安装或资源不完整\n\n错误详情: ${err.message}\n\n请先通过插件商店安装插件包。`,
            );
          } else {
            setError(`插件 "${pluginId}" 资源加载失败`);
          }
        }
      } catch (err) {
        console.error("[PluginPlaceholder] 加载插件失败:", err);
        setError(`加载插件失败: ${err}`);
      } finally {
        setLoading(false);
      }
    };

    loadPlugin();
  }, [pluginId]);

  if (loading) {
    return (
      <div style={{ padding: "40px", textAlign: "center" }}>
        <div style={{ fontSize: "48px", marginBottom: "20px" }}>⏳</div>
        <h2>正在加载插件...</h2>
        <p style={{ color: "#7f8c8d" }}>请稍候</p>
      </div>
    );
  }

  if (error) {
    return (
      <div
        style={{
          padding: "40px",
          maxWidth: "600px",
          margin: "0 auto",
          textAlign: "center",
        }}
      >
        <div style={{ fontSize: "64px", marginBottom: "20px" }}>⚠️</div>
        <h2>插件加载失败</h2>
        <pre
          style={{
            textAlign: "left",
            background: "#f8f9fa",
            padding: "20px",
            borderRadius: "8px",
            fontSize: "14px",
            color: "#495057",
            whiteSpace: "pre-wrap",
            marginTop: "20px",
          }}
        >
          {error}
        </pre>
        <button
          onClick={() => setSelectedPlugin(null)}
          style={{
            marginTop: "20px",
            padding: "10px 20px",
            background: "#0078d4",
            color: "white",
            border: "none",
            borderRadius: "6px",
            cursor: "pointer",
            fontSize: "16px",
          }}
        >
          返回
        </button>
      </div>
    );
  }

  // 使用 iframe 的 srcdoc 加载插件前端
  if (htmlContent) {
    return (
      <iframe
        key={`plugin-iframe-${pluginId}`}
        ref={iframeRef}
        srcDoc={htmlContent}
        onLoad={() => {
          // iframe 加载完成后注入 pluginAPI
          if (iframeRef.current?.contentWindow) {
            iframeRef.current.contentWindow.window.pluginAPI = {
              call: async (
                pluginId: string,
                method: string,
                params: Record<string, unknown>,
              ) => {
                console.log(`[PluginAPI] 调用 ${pluginId}.${method}`, params);
                return await invoke("call_plugin_method", {
                  pluginId,
                  method,
                  params,
                });
              },
              get_plugin_config: async (pluginId: string) => {
                return await invoke("get_plugin_config", {
                  pluginId: pluginId,
                });
              },
              set_plugin_config: async (
                pluginId: string,
                config: Record<string, unknown>,
              ) => {
                return await invoke("set_plugin_config", {
                  pluginId: pluginId,
                  config,
                });
              },
              open_url: async (url: string) => {
                console.log(`[PluginAPI] 打开链接: ${url}`);
                return await invoke("open_url", { url });
              },
            };
            console.log("[PluginPlaceholder] pluginAPI 注入成功");
          }
        }}
        style={{
          width: "100%",
          height: "100%",
          border: "none",
          padding: 0,
          margin: 0,
          display: "block",
        }}
        title={`Plugin: ${pluginId}`}
      />
    );
  }

  return null;
}
