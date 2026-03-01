import { Component, For } from 'solid-js';
import './Sidebar.css';

interface PluginInfo {
  id: string;
  name: string;
  description: string;
  icon: string;
}

interface SidebarProps {
  plugins: PluginInfo[];
  selectedPlugin: string | null;
  onPluginSelect: (pluginId: string) => void;
}

const Sidebar: Component<SidebarProps> = (props) => {
  return (
    <div class="sidebar">
      <div class="sidebar-header">
        <h2>Work Tools</h2>
      </div>
      <div class="sidebar-content">
        <For each={props.plugins}>
          {(plugin) => (
            <div
              class={`sidebar-item ${props.selectedPlugin === plugin.id ? 'active' : ''}`}
              onClick={() => props.onPluginSelect(plugin.id)}
            >
              <div class="plugin-icon">{plugin.icon}</div>
              <div class="plugin-info">
                <div class="plugin-name">{plugin.name}</div>
                <div class="plugin-description">{plugin.description}</div>
              </div>
            </div>
          )}
        </For>
      </div>
    </div>
  );
};

export default Sidebar;
