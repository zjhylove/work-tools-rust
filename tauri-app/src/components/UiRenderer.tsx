import { Component, For, Switch, Match, createSignal } from 'solid-js';
import './UiRenderer.css';

interface UiField {
  type: string;
  label: string;
  key: string;
  placeholder?: string;
  default?: any;
  min?: number;
  max?: number;
  columns?: string[];
  data_binding?: string;
  options?: Array<{ label: string; value: string }>;
  action?: string;
}

interface ViewSchema {
  fields: UiField[];
}

interface UiRendererProps {
  schema: ViewSchema;
}

const UiRenderer: Component<UiRendererProps> = (props) => {
  const [values, setValues] = createSignal<Record<string, any>>({});

  const handleChange = (key: string, value: any) => {
    setValues((prev) => ({ ...prev, [key]: value }));
  };

  const handleAction = (action: string) => {
    console.log('Action:', action, 'Values:', values());
    // TODO: 调用插件方法
  };

  return (
    <div class="ui-renderer">
      <For each={props.schema.fields}>
        {(field) => (
          <div class="field-container">
            <Switch fallback={<div>Unknown field type: {field.type}</div>}>
              <Match when={field.type === 'input'}>
                <label class="field-label">{field.label}</label>
                <input
                  type="text"
                  class="field-input"
                  placeholder={field.placeholder}
                  value={values()[field.key] || field.default || ''}
                  onInput={(e) =>
                    handleChange(field.key, e.currentTarget.value)
                  }
                />
              </Match>

              <Match when={field.type === 'number'}>
                <label class="field-label">{field.label}</label>
                <input
                  type="number"
                  class="field-input"
                  min={field.min}
                  max={field.max}
                  value={values()[field.key] || field.default || ''}
                  onInput={(e) =>
                    handleChange(field.key, parseInt(e.currentTarget.value))
                  }
                />
              </Match>

              <Match when={field.type === 'checkbox'}>
                <label class="field-checkbox-label">
                  <input
                    type="checkbox"
                    checked={values()[field.key] || field.default || false}
                    onChange={(e) =>
                      handleChange(field.key, e.currentTarget.checked)
                    }
                  />
                  <span>{field.label}</span>
                </label>
              </Match>

              <Match when={field.type === 'select'}>
                <label class="field-label">{field.label}</label>
                <select
                  class="field-select"
                  value={values()[field.key] || field.default || ''}
                  onChange={(e) =>
                    handleChange(field.key, e.currentTarget.value)
                  }
                >
                  <For each={field.options || []}>
                    {(option) => (
                      <option value={option.value}>{option.label}</option>
                    )}
                  </For>
                </select>
              </Match>

              <Match when={field.type === 'button'}>
                <button
                  class="field-button"
                  onClick={() => handleAction(field.action || '')}
                >
                  {field.label}
                </button>
              </Match>

              <Match when={field.type === 'table'}>
                <label class="field-label">{field.label}</label>
                <div class="field-table">
                  <table>
                    <thead>
                      <tr>
                        <For each={field.columns || []}>
                          {(col) => <th>{col}</th>}
                        </For>
                      </tr>
                    </thead>
                    <tbody>
                      <tr>
                        <td
                          colSpan={field.columns?.length || 1}
                          class="empty-table"
                        >
                          暂无数据
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </Match>
            </Switch>
          </div>
        )}
      </For>
    </div>
  );
};

export default UiRenderer;
