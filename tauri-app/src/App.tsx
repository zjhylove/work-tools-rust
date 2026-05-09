import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import PluginStore from "./components/PluginStore";
import PluginPlaceholder from "./components/PluginPlaceholder";
import ErrorBoundary from "./components/ErrorBoundary";
import LogViewer from "./components/LogViewer";
import { devError, devLog } from "./utils/logger";
import { isTauri } from "./utils/env";
import {
  IconTerminal,
  IconPackage,
  IconX,
  IconCode,
  IconSun,
  IconMoon,
} from "./components/icons";
import type { PluginInfo } from "./types/plugin";

// Must match const EVENT_PLUGINS_READY in lib.rs
const EVENT_PLUGINS_READY = "plugins-ready";

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

export default function App() {
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedPlugin, setSelectedPlugin] = useState<string | null>(null);
  const [visitedPlugins, setVisitedPlugins] = useState<string[]>([]);
  const [showLogs, setShowLogs] = useState(false);
  const [showPluginMarket, setShowPluginMarket] = useState(false);
  const [theme, setTheme] = useState<"light" | "dark">(() => {
    const stored = localStorage.getItem("theme");
    return stored === "dark" ? "dark" : "light";
  });

  const loadPlugins = useCallback(async () => {
    try {
      if (!isTauri()) {
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
            icon: "🔢",
          },
        ];
        setPlugins(mockPlugins);
        setSelectedPlugin((prev) => prev ?? mockPlugins[0].id);
        setVisitedPlugins((prev) => (prev.length === 0 ? [mockPlugins[0].id] : prev));
        return;
      }

      const installedPlugins = await safeInvoke<PluginInfo[]>(
        "get_installed_plugins",
      );

      if (!Array.isArray(installedPlugins)) {
        devError("get_installed_plugins 返回了非预期类型:", typeof installedPlugins);
        return;
      }

      devLog(`加载了 ${installedPlugins.length} 个插件`);
      setPlugins(installedPlugins);

      if (installedPlugins.length === 0) {
        setSelectedPlugin(null);
        setVisitedPlugins([]);
      } else {
        const ids = installedPlugins.map((p) => p.id);
        setSelectedPlugin((prev) =>
          prev && ids.includes(prev) ? prev : ids[0],
        );
        setVisitedPlugins((prev) => {
          const valid = prev.filter((id) => ids.includes(id));
          return valid.length > 0 ? valid : [ids[0]];
        });
      }
    } catch (error) {
      devError("加载插件失败:", error);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadPlugins();
    if (!isTauri()) return;

    const p = listen(EVENT_PLUGINS_READY, () => {
      devLog("收到 plugins-ready 事件");
      loadPlugins();
    });

    return () => { p.then((fn) => fn()); };
  }, [loadPlugins]);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.querySelectorAll("iframe").forEach((iframe) => {
      iframe.contentWindow?.postMessage({ type: "theme", theme }, "*");
    });
    if (isTauri()) {
      invoke("set_window_theme", { theme }).catch((e) =>
        devError("set_window_theme failed:", e),
      );
    }
  }, [theme]);

  const MAX_CACHED = 5;

  const openPlugin = (pluginId: string) => {
    devLog("打开插件:", pluginId);
    setSelectedPlugin(pluginId);
    setVisitedPlugins((prev) => {
      const idx = prev.indexOf(pluginId);
      if (idx !== -1) {
        return [...prev.slice(0, idx), ...prev.slice(idx + 1), pluginId];
      }
      if (prev.length >= MAX_CACHED) {
        return [...prev.slice(1), pluginId];
      }
      return [...prev, pluginId];
    });
  };

  const toggleTheme = () => {
    setTheme((prev) => {
      const next = prev === "light" ? "dark" : "light";
      localStorage.setItem("theme", next);
      return next;
    });
  };

  return (
    <div className="app-container">
      <aside className="sidebar">
        <nav className="sidebar-list">
          {loading
            ? Array.from({ length: 6 }).map((_, i) => (
                <div key={`skel-${i}`} className="sidebar-item sidebar-skeleton">
                  <div className="sidebar-item-icon skeleton-shimmer" />
                  <div className="sidebar-item-body">
                    <span className="skeleton-line skeleton-line--name skeleton-shimmer" />
                    <span className="skeleton-line skeleton-line--desc skeleton-shimmer" />
                  </div>
                </div>
              ))
            : plugins.map((plugin, i) => (
                <div
                  key={plugin.id}
                  className={`sidebar-item sidebar-item--reveal${selectedPlugin === plugin.id ? " active" : ""}`}
                  title={plugin.description}
                  onClick={() => openPlugin(plugin.id)}
                  style={{ animationDelay: `${i * 40}ms` }}
                >
                  <div className="sidebar-item-icon">
                    <span className="emoji">{plugin.icon}</span>
                  </div>
                  <div className="sidebar-item-body">
                    <span className="sidebar-item-name">{plugin.name}</span>
                    <span className="sidebar-item-desc">{plugin.description}</span>
                  </div>
                </div>
              ))}
        </nav>

        <div className="sidebar-footer">
          <button
            className="sidebar-footer-btn"
            title="系统日志"
            onClick={() => setShowLogs(true)}
          >
            <IconTerminal size={20} />
          </button>
          <button
            className="sidebar-footer-btn"
            title={theme === "light" ? "切换暗色主题" : "切换亮色主题"}
            onClick={toggleTheme}
          >
            {theme === "light" ? <IconMoon size={20} /> : <IconSun size={20} />}
          </button>
          <button
            className="sidebar-footer-btn"
            title="插件市场"
            onClick={() => setShowPluginMarket(true)}
          >
            <IconPackage size={20} />
          </button>
        </div>
      </aside>

      <main className="content-area">
        {loading ? (
          <div className="app-loading">
            <div className="app-loading-logo">
              <IconCode size={36} />
            </div>
            <div className="app-loading-bar-track">
              <div className="app-loading-bar" />
            </div>
          </div>
        ) : (
          <>
            {visitedPlugins.map((pluginId) => (
              <div
                key={pluginId}
                className={`content-pane${pluginId === selectedPlugin ? "" : " content-pane--hidden"}`}
              >
                <ErrorBoundary>
                  <PluginPlaceholder
                    pluginId={pluginId}
                    setSelectedPlugin={setSelectedPlugin}
                    theme={theme}
                  />
                </ErrorBoundary>
              </div>
            ))}

            {!selectedPlugin && (
              <div className="welcome-screen">
                <div className="welcome-icon">
                  <IconCode size={36} />
                </div>
                <h2 className="welcome-title">欢迎使用 Work Tools</h2>
                <p className="welcome-subtitle">
                  请从左侧选择一个插件开始使用
                </p>
              </div>
            )}
          </>
        )}
      </main>

      {showLogs && <LogViewer onClose={() => setShowLogs(false)} />}

      {showPluginMarket && (
        <div className="market-overlay" onClick={() => setShowPluginMarket(false)}>
          <div className="market-modal" onClick={(e) => e.stopPropagation()}>
            <div className="market-header">
              <h3>插件市场</h3>
              <button
                className="market-close"
                onClick={() => setShowPluginMarket(false)}
              >
                <IconX size={18} />
              </button>
            </div>
            <div className="market-body">
              <PluginStore onPluginsChange={loadPlugins} />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
