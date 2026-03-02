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
          const html = await invoke<string>("read_plugin_asset", {
            pluginId: pluginId,
            assetPath: "index.html",
          });

          // 读取 main.js 内容
          const mainJs = await invoke<string>("read_plugin_asset", {
            pluginId: pluginId,
            assetPath: "main.js",
          });

          // 读取 styles.css 内容
          const styles = await invoke<string>("read_plugin_asset", {
            pluginId: pluginId,
            assetPath: "styles.css",
          });

          console.log("[PluginPlaceholder] 插件资源读取成功");

          // 组合完整的 HTML 内容
          const fullHtml = html
            .replace("./main.js", "data:text/javascript;base64," + btoa(mainJs))
            .replace("./styles.css", "data:text/css;base64," + btoa(styles));

          setHtmlContent(fullHtml);

          // 设置 window.pluginAPI 供插件使用
          setTimeout(() => {
            if (iframeRef.current) {
              const iframe = iframeRef.current;
              try {
                // 向 iframe 注入 pluginAPI
                if (iframe.contentWindow) {
                  iframe.contentWindow.window.pluginAPI = {
                    call: async (
                      method: string,
                      params: Record<string, unknown>,
                    ) => {
                      console.log(
                        `[PluginAPI] 调用 ${pluginId}.${method}`,
                        params,
                      );
                      return await invoke("call_plugin_method", {
                        pluginId,
                        method,
                        params,
                      });
                    },
                    get_plugin_config: async (id: string) => {
                      return await invoke("get_plugin_config", {
                        pluginId: id,
                      });
                    },
                    set_plugin_config: async (
                      id: string,
                      config: Record<string, unknown>,
                    ) => {
                      return await invoke("set_plugin_config", {
                        pluginId: id,
                        config,
                      });
                    },
                  };
                  console.log("[PluginPlaceholder] pluginAPI 注入成功");
                }
              } catch (err) {
                console.error("[PluginPlaceholder] 注入 pluginAPI 失败:", err);
              }
            }
          }, 100);
        } catch (err) {
          console.error("[PluginPlaceholder] 读取插件资源失败:", err);
          setError(`插件 "${pluginId}" 资源加载失败: ${err}`);
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
        ref={iframeRef}
        srcDoc={htmlContent}
        style={{
          width: "100%",
          height: "100%",
          border: "none",
          padding: 0,
          margin: 0,
        }}
        title={`Plugin: ${pluginId}`}
      />
    );
  }

  return null;
}
