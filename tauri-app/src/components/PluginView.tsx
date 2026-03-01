import { createSignal, onMount, Show, Setter } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { createPluginBridge } from "../utils/pluginBridge";
import "./PluginView.css";

interface PluginViewProps {
  pluginId: string;
  setSelectedPlugin: Setter<string | null>;
}

export default (props: PluginViewProps) => {
  const [html, setHtml] = createSignal<string>("");
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string>("");

  onMount(async () => {
    try {
      setLoading(true);
      setError("");

      const viewHtml = await invoke<string>("get_plugin_view", {
        pluginId: props.pluginId,
      });

      setHtml(viewHtml);

      // 创建插件桥并暴露到 window
      const bridge = createPluginBridge(props.pluginId);
      bridge.exposeToWindow();

      setLoading(false);
    } catch (err) {
      setError(err as string);
      setLoading(false);
    }
  });

  return (
    <div class="plugin-view">
      <Show when={loading()}>
        <div class="loading">加载中...</div>
      </Show>

      <Show when={error()}>
        <div class="error">加载失败: {error()}</div>
      </Show>

      <Show when={!loading() && !error() && html()}>
        <div innerHTML={html()} class="plugin-content" />
      </Show>
    </div>
  );
};
