import { For, Show, createSignal, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import AuthPlugin from "./components/AuthPlugin";
import PasswordManager from "./components/PasswordManager";
import { devError, devLog, devWarn } from "./utils/logger";
import "./App.css";

// 安全的 invoke 包装函数 - Tauri 2.x 的 invoke 函数会自动处理环境检测
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

function App() {
  const [plugins, setPlugins] = createSignal<PluginInfo[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [selectedPlugin, setSelectedPlugin] = createSignal<string | null>(null);

  // 对话框状态
  const [showLogs, setShowLogs] = createSignal(false);
  const [showPluginMarket, setShowPluginMarket] = createSignal(false);

  // 加载插件列表
  onMount(async () => {
    // 检查是否在 Tauri 环境中
    const tauriAvailable =
      typeof window !== "undefined" && "__TAURI__" in window;
    devLog("Tauri 环境检查:", tauriAvailable);

    // 如果不在 Tauri 环境,使用模拟数据
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

        // 默认选中第一个插件
        if (installedPlugins.length > 0) {
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

      // 降级处理:至少显示密码管理器
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
  });

  const openPlugin = async (pluginId: string) => {
    devLog("打开插件:", pluginId);
    setSelectedPlugin(pluginId);

    // 插件组件自己负责数据加载和验证
    // App.tsx 只负责路由
  };

  return (
    <div
      style={{
        display: "flex",
        height: "100vh",
        "font-family": "Arial, sans-serif",
        margin: 0,
        padding: 0,
        overflow: "hidden",
      }}
    >
      {/* 左侧侧边栏 */}
      <div
        class="sidebar-container"
        style={{
          width: "260px",
          display: "flex",
          "flex-direction": "column",
          "flex-shrink": 0,
        }}
      >
        {/* 插件列表 */}
        <Show when={!loading()}>
          <div
            style={{
              flex: 1,
              overflow: "auto",
              padding: "8px",
            }}
          >
            <For each={plugins()}>
              {(plugin) => (
                <div
                  onClick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    devLog("点击了插件:", plugin.id, plugin.name);
                    openPlugin(plugin.id);
                  }}
                  style={{
                    padding: "12px 14px",
                    cursor: "pointer",
                    "user-select": "none",
                    "border-radius": "8px",
                    margin: "0 0 4px 0",
                    background:
                      selectedPlugin() === plugin.id
                        ? "var(--accent-light)"
                        : "transparent",
                    border:
                      selectedPlugin() === plugin.id
                        ? "1px solid var(--accent)"
                        : "1px solid transparent",
                    transition: "all 0.15s ease",
                    color:
                      selectedPlugin() === plugin.id
                        ? "var(--accent)"
                        : "var(--text-primary)",
                  }}
                  onMouseEnter={(e) => {
                    if (selectedPlugin() !== plugin.id) {
                      e.currentTarget.style.background = "var(--hover-bg)";
                    }
                  }}
                  onMouseLeave={(e) => {
                    if (selectedPlugin() !== plugin.id) {
                      e.currentTarget.style.background = "transparent";
                    }
                  }}
                >
                  <div
                    style={{
                      display: "flex",
                      "align-items": "center",
                      gap: "12px",
                    }}
                  >
                    <span
                      style={{
                        "font-size": "28px",
                        width: "40px",
                        height: "40px",
                        display: "flex",
                        "align-items": "center",
                        "justify-content": "center",
                        background:
                          selectedPlugin() === plugin.id
                            ? "var(--accent)"
                            : "var(--bg-tertiary)",
                        "border-radius": "8px",
                      }}
                    >
                      {plugin.icon}
                    </span>
                    <div style={{ flex: 1 }}>
                      <div
                        style={{
                          "font-size": "14px",
                          "font-weight": "600",
                          "margin-bottom": "3px",
                        }}
                      >
                        {plugin.name}
                      </div>
                      <div
                        style={{
                          "font-size": "12px",
                          color: "var(--text-secondary)",
                          overflow: "hidden",
                          "text-overflow": "ellipsis",
                          "white-space": "nowrap",
                        }}
                      >
                        {plugin.description}
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </For>
          </div>
        </Show>

        {/* 底部工具栏 */}
        <div
          style={{
            padding: "12px 16px",
            "border-top": "1px solid var(--border-color)",
            display: "flex",
            "justify-content": "center",
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
              "border-radius": "10px",
              "font-size": "20px",
              transition: "all 0.2s",
              display: "flex",
              "align-items": "center",
              "justify-content": "center",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = "var(--hover-bg)";
              e.currentTarget.style.transform = "scale(1.05)";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "var(--bg-tertiary)";
              e.currentTarget.style.transform = "scale(1)";
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
              "border-radius": "10px",
              "font-size": "20px",
              transition: "all 0.2s",
              display: "flex",
              "align-items": "center",
              "justify-content": "center",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = "var(--hover-bg)";
              e.currentTarget.style.transform = "scale(1.05)";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "var(--bg-tertiary)";
              e.currentTarget.style.transform = "scale(1)";
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
          "flex-direction": "column",
        }}
      >
        <Show when={selectedPlugin() === "password-manager"}>
          <PasswordManager />
        </Show>

        {/* Auth Plugin 界面 */}
        <Show when={selectedPlugin() === "auth"}>
          <AuthPlugin />
        </Show>

        {/* 其他插件提示 */}
        <Show
          when={
            selectedPlugin() &&
            selectedPlugin() !== "password-manager" &&
            selectedPlugin() !== "auth"
          }
        >
          <div
            style={{
              padding: "40px",
              "text-align": "center",
              color: "#7f8c8d",
            }}
          >
            <div style={{ "font-size": "64px", "margin-bottom": "20px" }}>
              🚧
            </div>
            <h2 style={{ "font-size": "24px", margin: "0 0 10px 0" }}>
              插件开发中
            </h2>
            <p>该插件正在开发中,敬请期待...</p>
          </div>
        </Show>
      </div>

      {/* 日志对话框 */}
      <Show when={showLogs()}>
        <div
          style={{
            position: "fixed",
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: "rgba(0,0,0,0.5)",
            display: "flex",
            "align-items": "center",
            "justify-content": "center",
            "z-index": 1000,
          }}
        >
          <div
            style={{
              background: "white",
              "border-radius": "8px",
              width: "800px",
              height: "600px",
              "box-shadow": "0 4px 20px rgba(0,0,0,0.3)",
              display: "flex",
              "flex-direction": "column",
            }}
          >
            <div
              style={{
                padding: "20px",
                "border-bottom": "1px solid #dee2e6",
                display: "flex",
                "justify-content": "space-between",
                "align-items": "center",
              }}
            >
              <h3 style={{ margin: 0 }}>系统日志</h3>
              <button
                onClick={() => setShowLogs(false)}
                style={{
                  background: "transparent",
                  border: "none",
                  "font-size": "20px",
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
                "font-family": "monospace",
                "font-size": "13px",
                "line-height": "1.6",
              }}
            >
              <div>[INFO] Work Tools 应用启动成功</div>
              <div>[INFO] 插件管理器初始化完成</div>
              <div>[INFO] 发现 {plugins().length} 个已安装插件</div>
              <div>[INFO] 密码管理器加载成功</div>
            </div>
          </div>
        </div>
      </Show>

      {/* 插件市场对话框 */}
      <Show when={showPluginMarket()}>
        <div
          style={{
            position: "fixed",
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: "rgba(0,0,0,0.5)",
            display: "flex",
            "align-items": "center",
            "justify-content": "center",
            "z-index": 1000,
          }}
        >
          <div
            style={{
              background: "white",
              "border-radius": "8px",
              width: "600px",
              height: "400px",
              "box-shadow": "0 4px 20px rgba(0,0,0,0.3)",
              display: "flex",
              "flex-direction": "column",
            }}
          >
            <div
              style={{
                padding: "20px",
                "border-bottom": "1px solid #dee2e6",
                display: "flex",
                "justify-content": "space-between",
                "align-items": "center",
              }}
            >
              <h3 style={{ margin: 0 }}>插件市场</h3>
              <button
                onClick={() => setShowPluginMarket(false)}
                style={{
                  background: "transparent",
                  border: "none",
                  "font-size": "20px",
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
              }}
            >
              <For each={plugins()}>
                {(plugin) => (
                  <div
                    style={{
                      padding: "15px",
                      margin: "0 0 10px 0",
                      background: "#f8f9fa",
                      "border-radius": "4px",
                      border: "1px solid #dee2e6",
                    }}
                  >
                    <div
                      style={{
                        display: "flex",
                        "align-items": "center",
                        gap: "15px",
                      }}
                    >
                      <span style={{ "font-size": "32px" }}>{plugin.icon}</span>
                      <div style={{ flex: 1 }}>
                        <div
                          style={{
                            "font-weight": "600",
                            "margin-bottom": "5px",
                          }}
                        >
                          {plugin.name}
                        </div>
                        <div style={{ "font-size": "13px", color: "#666" }}>
                          {plugin.description}
                        </div>
                        <div
                          style={{
                            "font-size": "12px",
                            color: "#999",
                            "margin-top": "5px",
                          }}
                        >
                          版本: {plugin.version}
                        </div>
                      </div>
                      <div
                        style={{
                          padding: "4px 12px",
                          background: "#27ae60",
                          color: "white",
                          "border-radius": "4px",
                          "font-size": "12px",
                        }}
                      >
                        已安装
                      </div>
                    </div>
                  </div>
                )}
              </For>
            </div>
          </div>
        </div>
      </Show>
    </div>
  );
}

export default App;
