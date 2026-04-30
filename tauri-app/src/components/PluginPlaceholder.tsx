import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface PluginPlaceholderProps {
  pluginId: string;
  setSelectedPlugin: (pluginId: string | null) => void;
}

const INJECTED_TOKENS = `
  :root {
    --accent: #0066ff;
    --accent-hover: #0052cc;
    --accent-light: #eef3ff;
    --accent-ring: rgba(0, 102, 255, 0.15);
    --success: #10b981;
    --success-light: #ecfdf5;
    --success-border: #a7f3d0;
    --success-text: #059669;
    --warning: #f59e0b;
    --warning-light: #fffbeb;
    --warning-border: #fde68a;
    --warning-text: #b45309;
    --error: #ef4444;
    --error-light: #fef2f2;
    --error-border: #fecaca;
    --error-text: #b91c1c;
    --bg-primary: #ffffff;
    --bg-secondary: #f8f9fa;
    --bg-tertiary: #f1f3f5;
    --hover-bg: rgba(0, 0, 0, 0.04);
    --text-primary: #1b1c1d;
    --text-secondary: #6b7280;
    --text-tertiary: #9ca3af;
    --text-inverse: #ffffff;
    --border-color: #e5e7eb;
    --border-light: #f1f3f5;
    --shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.03);
    --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.05), 0 1px 2px rgba(0, 0, 0, 0.04);
    --shadow-md: 0 4px 12px rgba(0, 0, 0, 0.06), 0 2px 4px rgba(0, 0, 0, 0.04);
    --shadow-lg: 0 12px 32px rgba(0, 0, 0, 0.08), 0 4px 8px rgba(0, 0, 0, 0.04);
    --radius-xs: 4px;
    --radius-sm: 6px;
    --radius-md: 8px;
    --radius-lg: 12px;
    --radius-xl: 16px;
    --radius-2xl: 20px;
    --space-xs: 4px;
    --space-sm: 8px;
    --space-md: 12px;
    --space-lg: 16px;
    --space-xl: 24px;
    --space-2xl: 32px;
    --font-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans SC", "PingFang SC", "Microsoft YaHei", sans-serif;
    --font-mono: "SF Mono", "Cascadia Code", "Fira Code", "JetBrains Mono", Consolas, monospace;
    --font-size-xs: 11px;
    --font-size-sm: 12px;
    --font-size-base: 13px;
    --font-size-md: 14px;
    --font-size-lg: 16px;
    --font-size-xl: 18px;
    --transition-fast: 0.12s ease;
    --transition-base: 0.2s ease;
    --transition-slow: 0.3s ease;
  }
  * {
    box-sizing: border-box;
  }
  body {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans SC", "PingFang SC", "Microsoft YaHei", sans-serif;
    color: #1b1c1d;
    background: #ffffff;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }
  ::-webkit-scrollbar { width: 5px; height: 5px; }
  ::-webkit-scrollbar-track { background: transparent; }
  ::-webkit-scrollbar-thumb { background: var(--border-color); border-radius: 3px; }
  ::-webkit-scrollbar-thumb:hover { background: var(--text-tertiary); }
`;

export default function PluginPlaceholder({
  pluginId,
  setSelectedPlugin,
}: PluginPlaceholderProps) {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>("");
  const [htmlContent, setHtmlContent] = useState<string>("");
  const iframeRef = useRef<HTMLIFrameElement>(null);

  useEffect(() => {
    let cancelled = false;

    const loadPlugin = async () => {
      if (!pluginId) return;

      console.log("[PluginPlaceholder] 开始加载插件:", pluginId);
      try {
        setLoading(true);
        setError("");

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

        if (cancelled) return;

        console.log("[PluginPlaceholder] 插件资源读取成功");

        const parts = html.split(
          '<script type="module" crossorigin src="./main.js"></script>',
        );
        const fullHtml =
          parts[0] +
          `<style>${INJECTED_TOKENS}${styles}</style><script type="module">${mainJs}</script>` +
          parts
            .slice(1)
            .join(
              '<script type="module" crossorigin src="./main.js"></script>',
            )
            .split('<link rel="stylesheet" crossorigin href="./styles.css">')
            .join("");

        if (!cancelled) setHtmlContent(fullHtml);
      } catch (err) {
        if (cancelled) return;
        console.error("[PluginPlaceholder] 读取插件资源失败:", err);
        if (err instanceof Error) {
          setError(
            `插件 "${pluginId}" 未安装或资源不完整\n\n错误详情: ${err.message}\n\n请先通过插件商店安装插件包。`,
          );
        } else {
          setError(`插件 "${pluginId}" 资源加载失败`);
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    loadPlugin();
    return () => { cancelled = true; };
  }, [pluginId]);

  if (loading) {
    return (
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          height: "100%",
          gap: "16px",
          color: "var(--text-tertiary, #9ca3af)",
        }}
      >
        <div className="plugin-spinner" />
        <span style={{ fontSize: 14 }}>正在加载插件...</span>
      </div>
    );
  }

  if (error) {
    return (
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          height: "100%",
          padding: 40,
          maxWidth: 520,
          margin: "0 auto",
          textAlign: "center",
        }}
      >
        <div
          style={{
            width: 56,
            height: 56,
            borderRadius: "50%",
            background: "var(--error-light, #fef2f2)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            marginBottom: 16,
            fontSize: 24,
          }}
        >
          !
        </div>
        <h2 style={{ margin: "0 0 8px", fontSize: 17, fontWeight: 600 }}>
          插件加载失败
        </h2>
        <pre
          style={{
            textAlign: "left",
            background: "var(--bg-secondary, #f8f9fa)",
            padding: 16,
            borderRadius: "var(--radius-md, 8px)",
            fontSize: 13,
            color: "var(--text-secondary, #6b7280)",
            whiteSpace: "pre-wrap",
            width: "100%",
            margin: "12px 0",
            fontFamily: "inherit",
          }}
        >
          {error}
        </pre>
        <button
          onClick={() => setSelectedPlugin(null)}
          style={{
            padding: "8px 20px",
            background: "var(--accent, #0066ff)",
            color: "white",
            border: "none",
            borderRadius: "var(--radius-md, 8px)",
            cursor: "pointer",
            fontSize: 14,
            fontWeight: 500,
            fontFamily: "inherit",
          }}
        >
          返回
        </button>
      </div>
    );
  }

  if (htmlContent) {
    return (
      <iframe
        key={`plugin-iframe-${pluginId}`}
        ref={iframeRef}
        srcDoc={htmlContent}
        onLoad={() => {
          if (iframeRef.current?.contentWindow) {
            const win = iframeRef.current.contentWindow.window as any;
            win.pluginAPI = {
              call: async (
                pId: string,
                method: string,
                params: Record<string, unknown>,
              ) => {
                console.log(`[PluginAPI] 调用 ${pId}.${method}`, params);
                return await invoke("call_plugin_method", {
                  pluginId: pId,
                  method,
                  params,
                });
              },
              get_plugin_config: async (pId: string) => {
                return await invoke("get_plugin_config", { pluginId: pId });
              },
              set_plugin_config: async (
                pId: string,
                config: Record<string, unknown>,
              ) => {
                return await invoke("set_plugin_config", {
                  pluginId: pId,
                  config,
                });
              },
              open_url: async (url: string) => {
                console.log(`[PluginAPI] 打开链接: ${url}`);
                return await invoke("open_url", { url });
              },
              open_folder_dialog: async (title?: string) => {
                console.log(`[PluginAPI] 打开文件夹对话框`);
                return await invoke("open_folder_dialog", {
                  title: title || "选择目录",
                });
              },
              open_file_dialog: async (title?: string) => {
                console.log(`[PluginAPI] 打开文件对话框`);
                return await invoke("open_file_dialog", {
                  title: title || "选择文件",
                });
              },
              write_file: async (path: string, content: string) => {
                console.log(`[PluginAPI] 写入文件: ${path}`);
                return await invoke("write_file", { path, content });
              },
            };
            console.log("[PluginPlaceholder] pluginAPI 注入成功");
          }
        }}
        style={{
          width: "100%",
          height: "100%",
          border: "none",
          display: "block",
        }}
        title={`Plugin: ${pluginId}`}
      />
    );
  }

  return null;
}
