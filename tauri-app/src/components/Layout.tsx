import { Component, createSignal, onMount, For, Show } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import Sidebar from './Sidebar';
import ContentArea from './ContentArea';
import Toolbar from './Toolbar';
import PluginMarket from './PluginMarket';
import LogViewer from './LogViewer';
import Settings from './Settings';
import './Layout.css';

interface PluginInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  icon: string;
}

const Layout: Component = () => {
  const [plugins, setPlugins] = createSignal<PluginInfo[]>([]);
  const [selectedPlugin, setSelectedPlugin] = createSignal<string | null>(null);
  const [currentView, setCurrentView] = createSignal<'plugin' | 'market' | 'log' | 'settings'>('plugin');
  const [showMarket, setShowMarket] = createSignal(false);
  const [showLog, setShowLog] = createSignal(false);
  const [showSettings, setShowSettings] = createSignal(false);

  onMount(async () => {
    await loadPlugins();
  });

  const loadPlugins = async () => {
    try {
      const installed = await invoke<PluginInfo[]>('get_installed_plugins');
      setPlugins(installed);

      // 自动选择第一个插件
      if (installed.length > 0) {
        setSelectedPlugin(installed[0].id);
      }
    } catch (error) {
      console.error('Failed to load plugins:', error);
    }
  };

  const handlePluginSelect = (pluginId: string) => {
    setSelectedPlugin(pluginId);
    setCurrentView('plugin');
  };

  const handleMarketClose = async () => {
    setShowMarket(false);
    await loadPlugins(); // 重新加载插件列表
  };

  return (
    <div class="layout">
      <Sidebar
        plugins={plugins()}
        selectedPlugin={selectedPlugin()}
        onPluginSelect={handlePluginSelect}
      />
      <div class="main-area">
        <Toolbar
          onOpenMarket={() => setShowMarket(true)}
          onOpenLog={() => setShowLog(true)}
          onOpenSettings={() => setShowSettings(true)}
        />
        <Show when={currentView() === 'plugin' && selectedPlugin()}>
          <ContentArea pluginId={selectedPlugin()!} />
        </Show>
      </div>

      {/* 对话框 */}
      <Show when={showMarket()}>
        <PluginMarket onClose={handleMarketClose} />
      </Show>

      <Show when={showLog()}>
        <LogViewer onClose={() => setShowLog(false)} />
      </Show>

      <Show when={showSettings()}>
        <Settings onClose={() => setShowSettings(false)} />
      </Show>
    </div>
  );
};

export default Layout;
