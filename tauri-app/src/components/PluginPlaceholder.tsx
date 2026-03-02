import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface PluginPlaceholderProps {
  pluginId: string;
  setSelectedPlugin: (pluginId: string | null) => void;
}

export default function PluginPlaceholder({
  pluginId,
  setSelectedPlugin,
}: PluginPlaceholderProps) {
  const [html, setHtml] = useState<string>("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>("");

  useEffect(() => {
    const loadPlugin = async () => {
      if (!pluginId) return;

      console.log("[PluginPlaceholder] 开始加载插件:", pluginId);
      try {
        setLoading(true);
        setError("");

        // 尝试读取插件的前端资源
        try {
          const indexHtml = await invoke<string>("read_plugin_asset", {
            pluginId: pluginId,
            assetPath: "index.html",
          });

          console.log("[PluginPlaceholder] 插件资源加载成功");
          setHtml(indexHtml);
        } catch (err) {
          // 插件没有前端资源,显示提示
          console.log("[PluginPlaceholder] 插件没有前端资源");
          setError(
            `插件 "${pluginId}" 尚未实现 React 前端组件。\n\n请参考以下文件实现:\n- /src/components/PasswordManagerReact.tsx\n- /src/components/AuthPluginReact.tsx`,
          );
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
        <div style={{ fontSize: "64px", marginBottom: "20px" }}>🚧</div>
        <h2>插件开发中</h2>
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

  // 如果有 HTML 内容,使用 dangerouslySetInnerHTML 渲染
  if (html) {
    return (
      <div
        style={{ padding: "20px", height: "100%", overflow: "auto" }}
        dangerouslySetInnerHTML={{ __html: html }}
      />
    );
  }

  return null;
}
