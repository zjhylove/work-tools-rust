import { createSignal, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import "./PluginStore.css";

interface PluginManifest {
  id: string;
  name: string;
  description: string;
  version: string;
  icon?: string;
  author?: string;
  homepage?: string;
}

interface InstalledPlugin {
  id: string;
  name: string;
  description: string;
  version: string;
  icon?: string;
  author?: string;
  homepage?: string;
  installed_at: string;
  enabled: boolean;
  assets_path: string;
  library_path: string;
}

interface PluginInfo extends PluginManifest {
  installed: boolean;
}

interface PluginStoreProps {
  onPluginsChange?: () => void;
}

export default function PluginStore(props: PluginStoreProps) {
  const [plugins, setPlugins] = createSignal<PluginInfo[]>([]);
  const [searchQuery, setSearchQuery] = createSignal("");
  const [loading, setLoading] = createSignal(true);
  const [importing, setImporting] = createSignal(false);

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
      const merged: PluginInfo[] = available.map((p) => ({
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
  const togglePlugin = async (plugin: PluginInfo) => {
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
  const filteredPlugins = () => {
    const query = searchQuery().toLowerCase();
    return plugins().filter(
      (p) =>
        p.name.toLowerCase().includes(query) ||
        p.description.toLowerCase().includes(query) ||
        p.id.toLowerCase().includes(query),
    );
  };

  // 组件挂载时加载插件列表
  loadPlugins();

  return (
    <div class="plugin-store">
      {/* 搜索栏 */}
      <div class="search-bar">
        <input
          type="text"
          placeholder="搜索插件..."
          value={searchQuery()}
          onInput={(e) => setSearchQuery(e.currentTarget.value)}
          class="search-input"
        />
        <button
          onClick={importPlugin}
          disabled={importing()}
          class="import-button"
        >
          {importing() ? "导入中..." : "导入插件"}
        </button>
      </div>

      {/* 加载状态 */}
      <Show when={loading()}>
        <div class="loading">加载中...</div>
      </Show>

      {/* 插件列表 */}
      <Show when={!loading()}>
        <div class="plugin-list">
          <Show
            when={filteredPlugins().length > 0}
            fallback={
              <div class="empty-state">
                {searchQuery() ? "未找到匹配的插件" : "暂无可用插件"}
              </div>
            }
          >
            <For each={filteredPlugins()}>
              {(plugin) => (
                <div class="plugin-card">
                  <div class="plugin-icon">{plugin.icon || "🔌"}</div>
                  <div class="plugin-info">
                    <h3 class="plugin-name">{plugin.name}</h3>
                    <p class="plugin-description">{plugin.description}</p>
                    <div class="plugin-meta">
                      <span class="version">v{plugin.version}</span>
                      <Show when={plugin.author}>
                        <span class="author">by {plugin.author}</span>
                      </Show>
                    </div>
                  </div>
                  <button
                    class={`action-button ${plugin.installed ? "uninstall" : "install"}`}
                    onClick={() => togglePlugin(plugin)}
                  >
                    {plugin.installed ? "卸载" : "安装"}
                  </button>
                </div>
              )}
            </For>
          </Show>
        </div>
      </Show>
    </div>
  );
}
