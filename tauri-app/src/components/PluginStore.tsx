import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import "./PluginStore.css";
import type { PluginManifest, InstalledPlugin, StorePluginInfo } from "../types/plugin";

interface PluginStoreProps {
  onPluginsChange?: () => void;
}

export default function PluginStore(props: PluginStoreProps) {
  const [plugins, setPlugins] = useState<StorePluginInfo[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [importing, setImporting] = useState(false);

  // 加载可用插件列表
  const loadPlugins = async () => {
    try {
      setLoading(true);
      const available = await invoke<PluginManifest[]>("get_available_plugins");
      const installed = await invoke<InstalledPlugin[]>(
        "get_installed_plugins_from_registry",
      );

      const installedIds = new Set(installed.map((p) => p.id));

      // 合并并标记安装状态
      const merged: StorePluginInfo[] = available.map((p) => ({
        ...p,
        installed: installedIds.has(p.id),
      }));

      setPlugins(merged);
    } catch (err) {
      console.error("加载插件列表失败:", err);
      alert("加载插件列表失败: " + err);
    } finally {
      setLoading(false);
    }
  };

  // 组件挂载时加载插件列表
  useEffect(() => {
    loadPlugins();
  }, []);

  // 导入插件包
  const importPlugin = async () => {
    try {
      setImporting(true);

      // 使用 Tauri 文件对话框选择插件包
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "插件包",
            extensions: ["zip", "wtplugin"],
          },
        ],
      });

      if (!selected) return;

      const result = await invoke<string>("import_plugin_package", {
        filePath: selected,
      });

      alert(result);
      await loadPlugins(); // 刷新列表

      // 通知父组件刷新插件列表
      if (props.onPluginsChange) {
        props.onPluginsChange();
      }
    } catch (err) {
      console.error("导入插件失败:", err);
      alert("导入插件失败: " + err);
    } finally {
      setImporting(false);
    }
  };

  // 安装/卸载插件
  const togglePlugin = async (plugin: StorePluginInfo) => {
    try {
      if (plugin.installed) {
        const result = await invoke<string>("uninstall_plugin", {
          pluginId: plugin.id,
        });
        alert(result);
      } else {
        const result = await invoke<string>("install_plugin", {
          pluginId: plugin.id,
        });
        alert(result);
      }
      await loadPlugins();

      // 通知父组件刷新插件列表
      if (props.onPluginsChange) {
        props.onPluginsChange();
      }
    } catch (err) {
      console.error("操作插件失败:", err);
      alert("操作插件失败: " + err);
    }
  };

  // 搜索过滤
  const filteredPlugins = useMemo(() => {
    const query = searchQuery.toLowerCase();
    return plugins.filter(
      (p) =>
        p.name.toLowerCase().includes(query) ||
        p.description.toLowerCase().includes(query) ||
        p.id.toLowerCase().includes(query),
    );
  }, [plugins, searchQuery]);

  return (
    <div className="plugin-store">
      {/* 搜索栏 */}
      <div className="search-bar">
        <input
          type="text"
          placeholder="搜索插件..."
          value={searchQuery}
          onInput={(e) => setSearchQuery(e.currentTarget.value)}
          className="search-input"
        />
        <button
          onClick={importPlugin}
          disabled={importing}
          className="import-button"
        >
          {importing ? "导入中..." : "导入插件"}
        </button>
      </div>

      {/* 加载状态 */}
      {loading && <div className="loading">加载中...</div>}

      {/* 插件列表 */}
      {!loading && (
        <div className="plugin-list">
          {filteredPlugins.length === 0 ? (
            <div className="empty-state">
              {searchQuery ? "未找到匹配的插件" : "暂无可用插件"}
            </div>
          ) : (
            filteredPlugins.map((plugin) => (
              <div key={plugin.id} className="plugin-card">
                <div className="plugin-icon">{plugin.icon || "🔌"}</div>
                <div className="plugin-info">
                  <h3 className="plugin-name">{plugin.name}</h3>
                  <p className="plugin-description">{plugin.description}</p>
                  <div className="plugin-meta">
                    <span className="version">v{plugin.version}</span>
                    {plugin.author && (
                      <span className="author">by {plugin.author}</span>
                    )}
                  </div>
                </div>
                <button
                  className={`action-button ${plugin.installed ? "uninstall" : "install"}`}
                  onClick={() => togglePlugin(plugin)}
                >
                  {plugin.installed ? "卸载" : "安装"}
                </button>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}
