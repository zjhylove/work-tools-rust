import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import PluginStore from "./components/PluginStore";
import PluginPlaceholder from "./components/PluginPlaceholder";
import ErrorBoundary from "./components/ErrorBoundary";
import LogViewer from "./components/LogViewer";
import { devError, devLog, devWarn } from "./utils/logger";
import {
  IconTerminal,
  IconPackage,
  IconX,
  IconCode,
} from "./components/icons";
import type { PluginInfo } from "./types/plugin";

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

  const loadPlugins = useCallback(async () => {
    const tauriAvailable =
      typeof window !== "undefined" && "__TAURI__" in window;
    devLog("Tauri 环境检查:", tauriAvailable);

    if (!tauriAvailable) {
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
        setSelectedPlugin((prev) =>
          prev ?? (installedPlugins.length > 0 ? installedPlugins[0].id : null),
        );
        setVisitedPlugins((prev) =>
          prev.length === 0 && installedPlugins.length > 0
            ? [installedPlugins[0].id]
            : prev,
        );
      }
    } catch (error) {
      devError("加载插件失败:", error);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadPlugins(); }, [loadPlugins]);

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

  return (
    <div className="app-container">
      {/* ── 侧边栏 ── */}
      <aside className="sidebar">
        {!loading && (
          <nav className="sidebar-list">
            {plugins.map((plugin) => (
              <div
                key={plugin.id}
                className={`sidebar-item${selectedPlugin === plugin.id ? " active" : ""}`}
                title={plugin.description}
                onClick={() => openPlugin(plugin.id)}
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
        )}

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
            title="插件市场"
            onClick={() => setShowPluginMarket(true)}
          >
            <IconPackage size={20} />
          </button>
        </div>
      </aside>

      {/* ── 内容区 ── */}
      <main className="content-area">
        {visitedPlugins.map((pluginId) => (
          <div
            key={pluginId}
            className={`content-pane${pluginId === selectedPlugin ? "" : " content-pane--hidden"}`}
          >
            <ErrorBoundary>
              <PluginPlaceholder
                pluginId={pluginId}
                setSelectedPlugin={setSelectedPlugin}
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
      </main>

      {/* ── 日志对话框 ── */}
      {showLogs && <LogViewer onClose={() => setShowLogs(false)} />}

      {/* ── 插件市场对话框 ── */}
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
