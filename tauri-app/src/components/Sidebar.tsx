import { Component, For } from "solid-js";
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
  // 添加调试日志
  console.log("=== Sidebar 渲染 ===");
  console.log("插件数量:", props.plugins.length);
  console.log("插件列表:", props.plugins);
  console.log("当前选中:", props.selectedPlugin);

  const handlePluginClick = (pluginId: string, event: MouseEvent) => {
    console.log("=== 插件点击事件 ===");
    console.log("点击的插件 ID:", pluginId);
    console.log("事件对象:", event);
    event.preventDefault();
    event.stopPropagation();
    console.log("调用 onPluginSelect...");
    props.onPluginSelect(pluginId);
    console.log("onPluginSelect 调用完成");
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
              class={`sidebar-item ${props.selectedPlugin === plugin.id ? "active" : ""}`}
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
