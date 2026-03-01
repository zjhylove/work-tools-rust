import { Component } from "solid-js";
import "./Toolbar.css";

interface ToolbarProps {
  onOpenMarket: () => void;
  onOpenLog: () => void;
}

const Toolbar: Component<ToolbarProps> = (props) => {
  return (
    <div class="toolbar">
      <button
        class="toolbar-button"
        onClick={props.onOpenMarket}
        title="插件市场"
      >
        🧩 插件市场
      </button>
      <button class="toolbar-button" onClick={props.onOpenLog} title="日志">
        📋 日志
      </button>
    </div>
  );
};

export default Toolbar;
