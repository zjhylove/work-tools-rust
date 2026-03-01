import { Component, createSignal, onMount } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import './Dialog.css';

interface PluginInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  icon: string;
}

interface PluginMarketProps {
  onClose: () => void;
}

const PluginMarket: Component<PluginMarketProps> = (props) => {
  const [availablePlugins, setAvailablePlugins] = createSignal<PluginInfo[]>([]);
  const [installing, setInstalling] = createSignal<string | null>(null);

  onMount(async () => {
    await loadAvailablePlugins();
  });

  const loadAvailablePlugins = async () => {
    try {
      const plugins = await invoke<PluginInfo[]>('get_available_plugins');
      setAvailablePlugins(plugins);
    } catch (error) {
      console.error('Failed to load available plugins:', error);
    }
  };

  const handleInstall = async (pluginId: string) => {
    try {
      setInstalling(pluginId);
      await invoke('install_plugin', { pluginId });
      await loadAvailablePlugins();
    } catch (error) {
      console.error('Failed to install plugin:', error);
      alert(`安装失败: ${error}`);
    } finally {
      setInstalling(null);
    }
  };

  return (
    <div class="dialog-overlay" onClick={props.onClose}>
      <div class="dialog-content" onClick={(e) => e.stopPropagation()}>
        <div class="dialog-header">
          <h2>插件市场</h2>
          <button class="dialog-close" onClick={props.onClose}>
            ✕
          </button>
        </div>
        <div class="dialog-body">
          <div class="plugin-list">
            <div class="plugin-empty">
              <p>暂无可用插件</p>
              <small>插件需要手动安装到 ~/.worktools/plugins/ 目录</small>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default PluginMarket;
