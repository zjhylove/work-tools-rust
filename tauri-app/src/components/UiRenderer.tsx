import { Component, For, Switch, Match, createSignal } from "solid-js";
import { devLog } from "../utils/logger";
import { UiField, ViewSchema } from "../types";
import "./UiRenderer.css";

interface UiRendererProps {
  schema: ViewSchema;
}

// 字段值的联合类型
type FieldValue = string | number | boolean | string[] | undefined;

const UiRenderer: Component<UiRendererProps> = (props) => {
  const [values, setValues] = createSignal<Record<string, FieldValue>>({});

  const handleChange = (key: string, value: FieldValue) => {
    setValues((prev) => ({ ...prev, [key]: value }));
  };

  const handleAction = (action: string) => {
    devLog("Action:", action, "Values:", values());
    // TODO: 调用插件方法
  };

  return (
    <div class="ui-renderer">
      <For each={props.schema.fields}>
        {(field) => (
          <div class="field-container">
            <Switch fallback={<div>Unknown field type: {field.type}</div>}>
              <Match when={field.type === "input"}>
                <label class="field-label">{field.label}</label>
                <input
                  type="text"
                  class="field-input"
                  placeholder={"placeholder" in field ? field.placeholder : ""}
                  value={
                    values()[field.key || ""] ||
                    ("default" in field ? field.default : "") ||
                    ""
                  }
                  onInput={(e) =>
                    handleChange(field.key || "", e.currentTarget.value)
                  }
                />
              </Match>

              <Match when={field.type === "number"}>
                <label class="field-label">{field.label}</label>
                <input
                  type="number"
                  class="field-input"
                  min={"min" in field ? field.min : undefined}
                  max={"max" in field ? field.max : undefined}
                  value={
                    values()[field.key || ""] ||
                    ("default" in field ? field.default : "") ||
                    ""
                  }
                  onInput={(e) =>
                    handleChange(
                      field.key || "",
                      parseInt(e.currentTarget.value),
                    )
                  }
                />
              </Match>

              <Match when={field.type === "checkbox"}>
                <label class="field-checkbox-label">
                  <input
                    type="checkbox"
                    checked={
                      !!(
                        values()[field.key || ""] ??
                        ("default" in field ? field.default : false)
                      )
                    }
                    onChange={(e) =>
                      handleChange(field.key || "", e.currentTarget.checked)
                    }
                  />
                  <span>{field.label}</span>
                </label>
              </Match>

              <Match when={field.type === "select"}>
                <label class="field-label">{field.label}</label>
                <select
                  class="field-select"
                  value={
                    values()[field.key || ""] ||
                    ("default" in field ? field.default : "") ||
                    ""
                  }
                  onChange={(e) =>
                    handleChange(field.key || "", e.currentTarget.value)
                  }
                >
                  <For each={"options" in field ? field.options : []}>
                    {(option) => (
                      <option value={option.value}>{option.label}</option>
                    )}
                  </For>
                </select>
              </Match>

              <Match when={field.type === "button"}>
                <button
                  class="field-button"
                  onClick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    handleAction("action" in field ? field.action || "" : "");
                  }}
                >
                  {field.label}
                </button>
              </Match>

              <Match when={field.type === "table"}>
                <label class="field-label">{field.label}</label>
                <div class="field-table">
                  <table>
                    <thead>
                      <tr>
                        <For each={"columns" in field ? field.columns : []}>
                          {(col) => (
                            <th>{typeof col === "string" ? col : ""}</th>
                          )}
                        </For>
                      </tr>
                    </thead>
                    <tbody>
                      <tr>
                        <td
                          colSpan={
                            "columns" in field ? field.columns?.length || 1 : 1
                          }
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
