import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import PluginStore from "./components/PluginStore";
import PluginPlaceholder from "./components/PluginPlaceholder";
import { devError, devLog, devWarn } from "./utils/logger";

// 安全的 invoke 包装函数
const safeInvoke = async <T,>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> => {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    devError("Invoke error:", error);
    throw error;
  }
};

interface PluginInfo {
  id: string;
  name: string;
  description: string;
  version: string;
  icon: string;
}

export default function App() {
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedPlugin, setSelectedPlugin] = useState<string | null>(null);
  const [showLogs, setShowLogs] = useState(false);
  const [showPluginMarket, setShowPluginMarket] = useState(false);

  // 加载插件列表
  useEffect(() => {
    const loadPlugins = async () => {
      const tauriAvailable =
        typeof window !== "undefined" && "__TAURI__" in window;
      devLog("Tauri 环境检查:", tauriAvailable);

      if (!tauriAvailable) {
        devWarn("不在 Tauri 环境,使用模拟数据");
        const mockPlugins: PluginInfo[] = [
          {
            id: "password-manager",
            name: "密码管理器",
            description: "本地安全存储和管理密码",
            version: "1.0.0",
            icon: "🔐",
          },
          {
            id: "auth",
            name: "双因素验证",
            description: "TOTP 双因素认证",
            version: "1.0.0",
            icon: "🔐",
          },
        ];
        setPlugins(mockPlugins);
        setSelectedPlugin(mockPlugins[0].id);
        setLoading(false);
        return;
      }

      try {
        const installedPlugins = await safeInvoke<PluginInfo[]>(
          "get_installed_plugins",
        );

        if (Array.isArray(installedPlugins)) {
          devLog(`加载了 ${installedPlugins.length} 个插件`);
          setPlugins(installedPlugins);

          if (!selectedPlugin && installedPlugins.length > 0) {
            setSelectedPlugin(installedPlugins[0].id);
          }
        } else {
          devError(
            "get_installed_plugins 返回的不是数组:",
            typeof installedPlugins,
          );
        }
      } catch (error) {
        devError("加载插件失败:", error);

        setPlugins([
          {
            id: "password-manager",
            name: "密码管理器",
            description: "本地安全存储和管理密码",
            version: "1.0.0",
            icon: "🔐",
          },
        ]);
      } finally {
        setLoading(false);
      }
    };

    loadPlugins();
  }, []);

  const openPlugin = async (pluginId: string) => {
    devLog("打开插件:", pluginId);
    setSelectedPlugin(pluginId);
  };

  // 所有插件统一使用 PluginPlaceholder 加载
  const renderPlugin = () => {
    if (!selectedPlugin) return null;

    return (
      <PluginPlaceholder
        pluginId={selectedPlugin}
        setSelectedPlugin={setSelectedPlugin}
      />
    );
  };

  return (
    <div
      style={
        {
          display: "flex",
          height: "100vh",
          fontFamily: "Arial, sans-serif",
          margin: 0,
          padding: 0,
          overflow: "hidden",
        } as React.CSSProperties
      }
    >
      {/* 左侧侧边栏 */}
      <div
        className="sidebar-container"
        style={
          {
            width: "260px",
            display: "flex",
            flexDirection: "column",
            flexShrink: 0,
          } as React.CSSProperties
        }
      >
        {/* 插件列表 */}
        {!loading && (
          <div
            style={{
              flex: 1,
              overflow: "auto",
              padding: "8px",
            }}
          >
            {plugins.map((plugin) => (
              <div
                key={plugin.id}
                onClick={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  devLog("点击了插件:", plugin.id, plugin.name);
                  openPlugin(plugin.id);
                }}
                style={{
                  padding: "12px 14px",
                  cursor: "pointer",
                  userSelect: "none",
                  borderRadius: "8px",
                  margin: "0 0 4px 0",
                  background:
                    selectedPlugin === plugin.id
                      ? "var(--accent-light)"
                      : "transparent",
                  border:
                    selectedPlugin === plugin.id
                      ? "1px solid var(--accent)"
                      : "1px solid transparent",
                  transition: "all 0.15s ease",
                  color:
                    selectedPlugin === plugin.id
                      ? "var(--accent)"
                      : "var(--text-primary)",
                }}
                onMouseEnter={(e) => {
                  if (selectedPlugin !== plugin.id) {
                    e.currentTarget.style.background = "var(--hover-bg)";
                  }
                }}
                onMouseLeave={(e) => {
                  if (selectedPlugin !== plugin.id) {
                    e.currentTarget.style.background = "transparent";
                  }
                }}
              >
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "12px",
                  }}
                >
                  <span
                    style={{
                      fontSize: "28px",
                      width: "40px",
                      height: "40px",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      background:
                        selectedPlugin === plugin.id
                          ? "var(--accent)"
                          : "var(--bg-tertiary)",
                      borderRadius: "8px",
                    }}
                  >
                    {plugin.icon}
                  </span>
                  <div style={{ flex: 1 }}>
                    <div
                      style={{
                        fontSize: "14px",
                        fontWeight: "600",
                        marginBottom: "3px",
                      }}
                    >
                      {plugin.name}
                    </div>
                    <div
                      style={{
                        fontSize: "12px",
                        color: "var(--text-secondary)",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                    >
                      {plugin.description}
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* 底部工具栏 */}
        <div
          style={{
            padding: "12px 16px",
            borderTop: "1px solid var(--border-color)",
            display: "flex",
            justifyContent: "center",
            gap: "12px",
          }}
        >
          <button
            onClick={() => setShowLogs(true)}
            title="查看系统日志"
            style={{
              width: "44px",
              height: "44px",
              background: "var(--bg-tertiary)",
              border: "1px solid var(--border-color)",
              color: "var(--text-primary)",
              cursor: "pointer",
              borderRadius: "10px",
              fontSize: "20px",
              transition: "all 0.2s",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            📋
          </button>
          <button
            onClick={() => setShowPluginMarket(true)}
            title="打开插件市场"
            style={{
              width: "44px",
              height: "44px",
              background: "var(--bg-tertiary)",
              border: "1px solid var(--border-color)",
              color: "var(--text-primary)",
              cursor: "pointer",
              borderRadius: "10px",
              fontSize: "20px",
              transition: "all 0.2s",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            🧩
          </button>
        </div>
      </div>

      {/* 右侧内容区 */}
      <div
        style={{
          flex: 1,
          background: "var(--bg-tertiary)",
          overflow: "auto",
          display: "flex",
          flexDirection: "column",
        }}
      >
        {renderPlugin()}

        {!selectedPlugin && (
          <div
            style={{
              padding: "40px",
              textAlign: "center",
              color: "#7f8c8d",
            }}
          >
            <div style={{ fontSize: "64px", marginBottom: "20px" }}>👋</div>
            <h2 style={{ fontSize: "24px", margin: "0 0 10px 0" }}>
              欢迎使用 Work Tools
            </h2>
            <p>请从左侧选择一个插件开始使用</p>
          </div>
        )}
      </div>

      {/* 日志对话框 */}
      {showLogs && (
        <div
          style={{
            position: "fixed",
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: "rgba(0,0,0,0.5)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 1000,
          }}
        >
          <div
            style={{
              background: "white",
              borderRadius: "8px",
              width: "800px",
              height: "600px",
              boxShadow: "0 4px 20px rgba(0,0,0,0.3)",
              display: "flex",
              flexDirection: "column",
            }}
          >
            <div
              style={{
                padding: "20px",
                borderBottom: "1px solid #dee2e6",
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
              }}
            >
              <h3 style={{ margin: 0 }}>系统日志</h3>
              <button
                onClick={() => setShowLogs(false)}
                style={{
                  background: "transparent",
                  border: "none",
                  fontSize: "20px",
                  cursor: "pointer",
                  color: "#999",
                }}
              >
                ✕
              </button>
            </div>
            <div
              style={{
                flex: 1,
                padding: "20px",
                overflow: "auto",
                background: "#1e1e1e",
                color: "#d4d4d4",
                fontFamily: "monospace",
                fontSize: "13px",
                lineHeight: "1.6",
              }}
            >
              <div>[INFO] Work Tools 应用启动成功</div>
              <div>[INFO] 插件管理器初始化完成</div>
              <div>[INFO] 发现 {plugins.length} 个已安装插件</div>
              <div>[INFO] 密码管理器加载成功</div>
            </div>
          </div>
        </div>
      )}

      {/* 插件市场对话框 */}
      {showPluginMarket && (
        <div
          style={{
            position: "fixed",
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: "rgba(0,0,0,0.5)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 1000,
          }}
        >
          <div
            style={{
              background: "white",
              borderRadius: "8px",
              width: "800px",
              height: "600px",
              boxShadow: "0 4px 20px rgba(0,0,0,0.3)",
              display: "flex",
              flexDirection: "column",
            }}
          >
            <div
              style={{
                padding: "20px",
                borderBottom: "1px solid #dee2e6",
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
              }}
            >
              <h3 style={{ margin: 0 }}>插件市场</h3>
              <button
                onClick={() => setShowPluginMarket(false)}
                style={{
                  background: "transparent",
                  border: "none",
                  fontSize: "20px",
                  cursor: "pointer",
                  color: "#999",
                }}
              >
                ✕
              </button>
            </div>
            <div
              style={{
                flex: 1,
                padding: "0",
                overflow: "auto",
              }}
            >
              <PluginStore onPluginsChange={() => {}} />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
