import { Component, For } from "solid-js";
import { devLog } from "../utils/logger";
import "./Sidebar.css";

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
  const handlePluginClick = (pluginId: string, event: MouseEvent) => {
    devLog("点击插件:", pluginId);
    event.preventDefault();
    event.stopPropagation();
    props.onPluginSelect(pluginId);
  };

  return (
    <div class="sidebar">
      <div class="sidebar-header">
        <h2>Work Tools</h2>
      </div>
      <div class="sidebar-content">
        <For each={props.plugins}>
          {(plugin) => (
            <div
              classList={{
                "sidebar-item": true,
                "active": props.selectedPlugin === plugin.id,
              }}
              onClick={(e) => handlePluginClick(plugin.id, e)}
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
