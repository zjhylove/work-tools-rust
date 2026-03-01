import { Component, createSignal, onMount, Show } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import UiRenderer from './UiRenderer';
import './ContentArea.css';

interface ContentAreaProps {
  pluginId: string;
}

interface ViewSchema {
  fields: UiField[];
}

interface UiField {
  type: string;
  [key: string]: any;
}

const ContentArea: Component<ContentAreaProps> = (props) => {
  const [schema, setSchema] = createSignal<ViewSchema | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  onMount(async () => {
    await loadViewSchema();
  });

  const loadViewSchema = async () => {
    try {
      setLoading(true);
      setError(null);

      // TODO: 调用插件获取 UI Schema
      // const schema = await invoke<ViewSchema>('get_plugin_view', {
      //   pluginId: props.pluginId,
      // });

      // 临时使用模拟数据
      const mockSchema: ViewSchema = {
        fields: [
          {
            type: 'input',
            label: '服务器地址',
            key: 'host',
            placeholder: '请输入服务器地址',
          },
          {
            type: 'number',
            label: '端口',
            key: 'port',
            default: 22,
            min: 1,
            max: 65535,
          },
          {
            type: 'button',
            label: '连接',
            key: 'connect',
            action: 'connect',
          },
        ],
      };

      setSchema(mockSchema);
    } catch (err) {
      setError(err as string);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div class="content-area">
      <Show when={loading()}>
        <div class="loading">加载中...</div>
      </Show>

      <Show when={error()}>
        <div class="error">{error()}</div>
      </Show>

      <Show when={schema()}>
        <div class="plugin-content">
          <UiRenderer schema={schema()!} />
        </div>
      </Show>
    </div>
  );
};

export default ContentArea;
