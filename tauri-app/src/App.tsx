import { For, Show, createSignal, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

interface PasswordEntry {
  id: string;
  url: string | null;
  service: string;
  username: string;
  password: string;
  created_at: string;
  updated_at: string;
}

interface PluginInfo {
  id: string;
  name: string;
  description: string;
  version: string;
  icon: string;
}

interface UiField {
  type: string;
  label: string;
  key: string;
  placeholder?: string;
  default?: any;
  inputType?: string;
  required?: boolean;
  minLength?: number;
  pattern?: string;
}

interface ViewSchema {
  fields: UiField[];
}

function App() {
  const [plugins, setPlugins] = createSignal<PluginInfo[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [selectedPlugin, setSelectedPlugin] = createSignal<string | null>(null);
  const [pluginView, setPluginView] = createSignal<ViewSchema | null>(null);
  const [formData, setFormData] = createSignal<Record<string, string>>({});
  const [formErrors, setFormErrors] = createSignal<Record<string, string>>({});
  const [passwordEntries, setPasswordEntries] = createSignal<PasswordEntry[]>(
    [],
  );
  const [selectedEntry, setSelectedEntry] = createSignal<PasswordEntry | null>(
    null,
  );
  const [searchQuery, setSearchQuery] = createSignal("");
  const [isEditMode, setIsEditMode] = createSignal(false);

  // 对话框状态
  const [showSettings, setShowSettings] = createSignal(false);
  const [showLogs, setShowLogs] = createSignal(false);
  const [showPluginMarket, setShowPluginMarket] = createSignal(false);
  const [showDiagnostics, setShowDiagnostics] = createSignal(false);
  const [diagnostics, setDiagnostics] = createSignal<string[]>([]);
  const [theme, setTheme] = createSignal("light");
  const [autoStart, setAutoStart] = createSignal(false);
  const [minimizeToTray, setMinimizeToTray] = createSignal(true);

  // 加载插件列表
  onMount(async () => {
    console.log("=== App onMount 开始 ===");
    try {
      console.log("调用 get_installed_plugins...");
      const installedPlugins = await invoke<PluginInfo[]>(
        "get_installed_plugins",
      );
      console.log("已安装插件 (原始):", installedPlugins);
      console.log("已安装插件数量:", installedPlugins.length);

      if (Array.isArray(installedPlugins)) {
        installedPlugins.forEach((plugin, index) => {
          console.log(`插件 ${index}:`, {
            id: plugin.id,
            name: plugin.name,
            description: plugin.description,
            icon: plugin.icon,
            version: plugin.version,
          });
        });
        setPlugins(installedPlugins);

        // 加载配置
        try {
          const config = await invoke<any>("get_app_config");
          console.log("加载配置:", config);
          if (config) {
            setTheme(config.theme || "light");
            setAutoStart(config.settings?.auto_start || false);
            setMinimizeToTray(config.settings?.minimize_to_tray !== false);
          }
        } catch (configError) {
          console.warn("加载配置失败:", configError);
        }

        // 默认选中第一个插件
        if (installedPlugins.length > 0) {
          console.log("默认选中第一个插件:", installedPlugins[0].id);
          await openPlugin(installedPlugins[0].id);
        } else {
          console.warn("没有已安装的插件!");
        }
      } else {
        console.error(
          "get_installed_plugins 返回的不是数组:",
          typeof installedPlugins,
          installedPlugins,
        );
      }
    } catch (error) {
      console.error("加载插件失败:", error);
      console.error("错误详情:", JSON.stringify(error, null, 2));

      // 降级处理:至少显示密码管理器
      setPlugins([
        {
          id: "password-manager",
          name: "密码管理器",
          description: "本地安全存储和管理密码",
          version: "1.0.0",
          icon: "🔐",
        },
      ]);
    } finally {
      console.log("=== App onMount 完成,设置 loading = false ===");
      setLoading(false);
    }
  });

  const openPlugin = async (pluginId: string) => {
    console.log("打开插件:", pluginId);
    setSelectedPlugin(pluginId);

    if (pluginId === "password-manager") {
      try {
        const entries = await invoke<PasswordEntry[]>("get_password_entries");
        console.log("加载到的密码条目:", entries);
        setPasswordEntries(entries);
        setSelectedEntry(null);
        setIsEditMode(false);
        setFormData({});
        setFormErrors({});
      } catch (error) {
        console.error("加载密码列表失败:", error);
        setPasswordEntries([]);
      }
    } else if (pluginId === "auth") {
      // Auth plugin 处理
      try {
        const entries = await invoke<any[]>("get_auth_entries");
        console.log("加载到的认证条目:", entries);
      } catch (error) {
        console.error("加载认证列表失败:", error);
      }
    }

    // 模拟 UI Schema
    const schema: ViewSchema = {
      fields: [
        {
          type: "input",
          label: "账号地址",
          key: "url",
          placeholder: "例如: https://google.com",
          required: false,
          pattern: "^https?://.+",
        },
        {
          type: "input",
          label: "服务名称",
          key: "service",
          placeholder: "例如: Google",
          required: true,
          minLength: 2,
        },
        {
          type: "input",
          label: "用户名/邮箱",
          key: "username",
          placeholder: "输入用户名或邮箱",
          required: true,
        },
        {
          type: "input",
          label: "密码",
          key: "password",
          placeholder: "输入密码",
          inputType: "password",
          required: true,
          minLength: 6,
        },
        {
          type: "button",
          label: "💾 保存密码",
          key: "save",
        },
      ],
    };

    setPluginView(schema);
  };

  const validateField = (field: UiField, value: string): string | null => {
    if (field.required && !value.trim()) {
      return `${field.label}不能为空`;
    }
    if (field.minLength && value.length < field.minLength) {
      return `${field.label}至少需要 ${field.minLength} 个字符`;
    }
    if (field.pattern && value) {
      const regex = new RegExp(field.pattern);
      if (!regex.test(value)) {
        return `${field.label}格式不正确`;
      }
    }
    return null;
  };

  const validateForm = (): boolean => {
    const errors: Record<string, string> = {};
    const fields = pluginView()?.fields || [];

    for (const field of fields) {
      if (field.type === "input") {
        const value = formData()[field.key] || "";
        const error = validateField(field, value);
        if (error) {
          errors[field.key] = error;
        }
      }
    }

    setFormErrors(errors);
    return Object.keys(errors).length === 0;
  };

  const isFormValid = () => {
    const fields = pluginView()?.fields || [];
    for (const field of fields) {
      if (field.type === "input" && field.required) {
        const value = formData()[field.key] || "";
        if (!value.trim()) return false;
        if (field.minLength && value.length < field.minLength) return false;
        if (field.pattern) {
          const regex = new RegExp(field.pattern);
          if (!regex.test(value)) return false;
        }
      }
    }
    return true;
  };

  const handleFieldChange = (key: string, value: string, field: UiField) => {
    setFormData((prev) => ({ ...prev, [key]: value }));

    const error = validateField(field, value);
    setFormErrors((prev) => {
      const newErrors = { ...prev };
      if (error) {
        newErrors[key] = error;
      } else {
        delete newErrors[key];
      }
      return newErrors;
    });
  };

  const handleAction = async (action: string) => {
    if (action === "save") {
      if (!validateForm()) {
        alert("请修正表单中的错误后再提交");
        return;
      }

      const data = formData();
      const entry: PasswordEntry = {
        id:
          isEditMode() && selectedEntry()
            ? selectedEntry()!.id
            : Date.now().toString(),
        url: data.url || null,
        service: data.service || "",
        username: data.username || "",
        password: data.password || "",
        created_at:
          isEditMode() && selectedEntry()
            ? selectedEntry()!.created_at
            : new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };

      try {
        await invoke("save_password_entry", { entry });
        const entries = await invoke<PasswordEntry[]>("get_password_entries");
        setPasswordEntries(entries);
        alert(isEditMode() ? "密码更新成功!" : "密码保存成功!");
        setFormData({});
        setFormErrors({});
        setSelectedEntry(null);
        setIsEditMode(false);
      } catch (error) {
        console.error("保存密码失败:", error);
        alert("保存密码失败: " + error);
      }
    }
  };

  const handleSelectEntry = (entry: PasswordEntry) => {
    setSelectedEntry(entry);
    setIsEditMode(true);
    setFormData({
      url: entry.url || "",
      service: entry.service,
      username: entry.username,
      password: entry.password,
    });
    setFormErrors({});
  };

  const handleAddNew = () => {
    setSelectedEntry(null);
    setIsEditMode(false);
    setFormData({});
    setFormErrors({});
  };

  const handleDeletePassword = async (id: string) => {
    if (!confirm("确定要删除这条密码记录吗?")) {
      return;
    }

    try {
      await invoke("delete_password_entry", { id });
      const entries = await invoke<PasswordEntry[]>("get_password_entries");
      setPasswordEntries(entries);

      if (selectedEntry()?.id === id) {
        setSelectedEntry(null);
        setIsEditMode(false);
        setFormData({});
        setFormErrors({});
      }

      alert("删除成功!");
    } catch (error) {
      console.error("删除密码失败:", error);
      alert("删除失败: " + error);
    }
  };

  const handleSaveSettings = async () => {
    try {
      await invoke("set_app_config", {
        config: {
          theme: theme(),
          window_state: {
            width: 1200,
            height: 800,
            x: 100,
            y: 100,
            is_maximized: false,
          },
          settings: {
            auto_start: autoStart(),
            minimize_to_tray: minimizeToTray(),
          },
        },
      });
      alert("设置保存成功!");
      setShowSettings(false);
    } catch (error) {
      console.error("保存设置失败:", error);
      alert("保存设置失败: " + error);
    }
  };

  const filteredEntries = () => {
    const query = searchQuery().toLowerCase();
    if (!query) return passwordEntries();
    return passwordEntries().filter(
      (entry) =>
        entry.service.toLowerCase().includes(query) ||
        entry.username.toLowerCase().includes(query) ||
        (entry.url && entry.url.toLowerCase().includes(query)),
    );
  };

  const runDiagnostics = async () => {
    const results: string[] = [];
    results.push("=== 开始诊断 ===");
    results.push(`时间: ${new Date().toISOString()}`);

    try {
      results.push("\n1. 测试 get_installed_plugins:");
      const installed = await invoke<PluginInfo[]>("get_installed_plugins");
      results.push(`   返回类型: ${typeof installed}`);
      results.push(`   是否为数组: ${Array.isArray(installed)}`);
      results.push(
        `   数组长度: ${Array.isArray(installed) ? installed.length : "N/A"}`,
      );

      if (Array.isArray(installed)) {
        installed.forEach((p, i) => {
          results.push(
            `   插件[${i}]: id=${p.id}, name=${p.name}, icon=${p.icon}`,
          );
        });
      } else {
        results.push(`   实际值: ${JSON.stringify(installed)}`);
      }

      results.push("\n2. 测试 get_available_plugins:");
      const available = await invoke<PluginInfo[]>("get_available_plugins");
      results.push(`   可用插件数量: ${available.length}`);

      results.push("\n3. 当前前端状态:");
      results.push(`   plugins() 数量: ${plugins().length}`);
      plugins().forEach((p, i) => {
        results.push(`   前端插件[${i}]: id=${p.id}, name=${p.name}`);
      });

      results.push("\n4. 当前选中插件:");
      results.push(`   selectedPlugin: ${selectedPlugin()}`);
    } catch (error) {
      results.push(`\n错误: ${error}`);
    }

    results.push("\n=== 诊断完成 ===");
    setDiagnostics(results);
    setShowDiagnostics(true);
  };

  return (
    <div
      style={{
        display: "flex",
        height: "100vh",
        "font-family": "Arial, sans-serif",
        margin: 0,
        padding: 0,
        overflow: "hidden",
      }}
    >
      {/* 左侧侧边栏 */}
      <div
        style={{
          width: "250px",
          display: "flex",
          "flex-direction": "column",
          background: "#1e1e1e",
          color: "white",
          "flex-shrink": 0,
        }}
      >
        {/* 标题 */}
        <div
          style={{
            padding: "20px 15px",
            "text-align": "center",
            "border-bottom": "1px solid #333",
          }}
        >
          <h2
            style={{
              margin: 0,
              "font-size": "18px",
              "font-weight": "600",
              color: "#ffffff",
              "letter-spacing": "0.5px",
            }}
          >
            Work Tools
          </h2>
        </div>

        {/* 插件列表 */}
        <Show when={!loading()}>
          <div
            style={{
              flex: 1,
              overflow: "auto",
              padding: "10px 0",
            }}
          >
            <For each={plugins()}>
              {(plugin) => (
                <div
                  onClick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    console.log("点击了插件:", plugin.id, plugin.name);
                    openPlugin(plugin.id);
                  }}
                  style={{
                    padding: "12px 15px",
                    cursor: "pointer",
                    "user-select": "none",
                    background:
                      selectedPlugin() === plugin.id
                        ? "#2d2d2d"
                        : "transparent",
                    "border-left":
                      selectedPlugin() === plugin.id
                        ? "3px solid #0078d4"
                        : "3px solid transparent",
                    transition: "all 0.15s ease",
                  }}
                  onMouseEnter={(e) => {
                    if (selectedPlugin() !== plugin.id) {
                      e.currentTarget.style.background = "#2a2a2a";
                    }
                  }}
                  onMouseLeave={(e) => {
                    if (selectedPlugin() !== plugin.id) {
                      e.currentTarget.style.background = "transparent";
                    }
                  }}
                >
                  <div
                    style={{
                      display: "flex",
                      "align-items": "center",
                      gap: "10px",
                    }}
                  >
                    <span
                      style={{
                        "font-size": "24px",
                        width: "32px",
                        height: "32px",
                        display: "flex",
                        "align-items": "center",
                        "justify-content": "center",
                      }}
                    >
                      {plugin.icon}
                    </span>
                    <div style={{ flex: 1 }}>
                      <div
                        style={{
                          "font-size": "14px",
                          "font-weight": "500",
                          "margin-bottom": "2px",
                        }}
                      >
                        {plugin.name}
                      </div>
                      <div
                        style={{
                          "font-size": "11px",
                          opacity: 0.7,
                          overflow: "hidden",
                          "text-overflow": "ellipsis",
                          "white-space": "nowrap",
                        }}
                      >
                        {plugin.description}
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </For>
          </div>
        </Show>

        {/* 底部工具栏 */}
        <div
          style={{
            padding: "10px",
            "border-top": "1px solid #333",
            display: "flex",
            "justify-content": "center",
            gap: "15px",
          }}
        >
          <button
            onClick={() => setShowSettings(true)}
            title="设置"
            style={{
              width: "36px",
              height: "36px",
              background: "transparent",
              border: "none",
              color: "white",
              cursor: "pointer",
              "border-radius": "4px",
              "font-size": "18px",
              transition: "background 0.2s",
            }}
            onMouseEnter={(e) =>
              (e.currentTarget.style.background = "rgba(255,255,255,0.1)")
            }
            onMouseLeave={(e) =>
              (e.currentTarget.style.background = "transparent")
            }
          >
            ⚙️
          </button>
          <button
            onClick={() => setShowLogs(true)}
            title="日志"
            style={{
              width: "36px",
              height: "36px",
              background: "transparent",
              border: "none",
              color: "white",
              cursor: "pointer",
              "border-radius": "4px",
              "font-size": "18px",
              transition: "background 0.2s",
            }}
            onMouseEnter={(e) =>
              (e.currentTarget.style.background = "rgba(255,255,255,0.1)")
            }
            onMouseLeave={(e) =>
              (e.currentTarget.style.background = "transparent")
            }
          >
            📋
          </button>
          <button
            onClick={() => setShowPluginMarket(true)}
            title="插件市场"
            style={{
              width: "36px",
              height: "36px",
              background: "transparent",
              border: "none",
              color: "white",
              cursor: "pointer",
              "border-radius": "4px",
              "font-size": "18px",
              transition: "background 0.2s",
            }}
            onMouseEnter={(e) =>
              (e.currentTarget.style.background = "rgba(255,255,255,0.1)")
            }
            onMouseLeave={(e) =>
              (e.currentTarget.style.background = "transparent")
            }
          >
            🧩
          </button>
          <button
            onClick={runDiagnostics}
            title="诊断"
            style={{
              width: "36px",
              height: "36px",
              background: "transparent",
              border: "none",
              color: "white",
              cursor: "pointer",
              "border-radius": "4px",
              "font-size": "18px",
              transition: "background 0.2s",
            }}
            onMouseEnter={(e) =>
              (e.currentTarget.style.background = "rgba(255,255,255,0.1)")
            }
            onMouseLeave={(e) =>
              (e.currentTarget.style.background = "transparent")
            }
          >
            🔍
          </button>
        </div>
      </div>

      {/* 右侧内容区 */}
      <div
        style={{
          flex: 1,
          background: "#f5f5f5",
          overflow: "auto",
          display: "flex",
          "flex-direction": "column",
        }}
      >
        <Show when={selectedPlugin() === "password-manager"}>
          <div
            style={{
              flex: 1,
              display: "flex",
              gap: "20px",
              padding: "20px",
              height: "100%",
              "box-sizing": "border-box",
            }}
          >
            {/* 左侧:密码列表 */}
            <div
              style={{
                flex: 1,
                display: "flex",
                "flex-direction": "column",
                background: "white",
                "border-radius": "6px",
                overflow: "hidden",
                "box-shadow": "0 1px 3px rgba(0,0,0,0.08)",
                border: "1px solid #e0e0e0",
              }}
            >
              {/* 工具栏 */}
              <div
                style={{
                  padding: "15px",
                  background: "#fafafa",
                  "border-bottom": "1px solid #e0e0e0",
                }}
              >
                <div
                  style={{
                    display: "flex",
                    gap: "10px",
                    "margin-bottom": "10px",
                  }}
                >
                  <button
                    onClick={handleAddNew}
                    style={{
                      padding: "8px 16px",
                      background: "#0078d4",
                      color: "white",
                      border: "none",
                      "border-radius": "3px",
                      cursor: "pointer",
                      "font-size": "13px",
                      "font-weight": "500",
                      transition: "background 0.15s",
                    }}
                    onMouseEnter={(e) =>
                      (e.currentTarget.style.background = "#106ebe")
                    }
                    onMouseLeave={(e) =>
                      (e.currentTarget.style.background = "#0078d4")
                    }
                  >
                    ➕ 新建
                  </button>
                  <button
                    style={{
                      padding: "8px 16px",
                      background: "#6c757d",
                      color: "white",
                      border: "none",
                      "border-radius": "3px",
                      cursor: "pointer",
                      "font-size": "13px",
                      "font-weight": "500",
                    }}
                  >
                    📥 导入
                  </button>
                  <button
                    style={{
                      padding: "8px 16px",
                      background: "#6c757d",
                      color: "white",
                      border: "none",
                      "border-radius": "3px",
                      cursor: "pointer",
                      "font-size": "13px",
                      "font-weight": "500",
                    }}
                  >
                    📤 导出
                  </button>
                </div>
                <input
                  type="text"
                  placeholder="🔍 搜索密码..."
                  value={searchQuery()}
                  onInput={(e) => setSearchQuery(e.currentTarget.value)}
                  style={{
                    width: "100%",
                    padding: "8px 12px",
                    border: "1px solid #d0d0d0",
                    "border-radius": "3px",
                    "font-size": "13px",
                    "font-family": "inherit",
                  }}
                />
              </div>

              {/* 密码列表 */}
              <div
                style={{
                  flex: 1,
                  overflow: "auto",
                  padding: "10px",
                }}
              >
                <Show when={filteredEntries().length === 0}>
                  <div
                    style={{
                      "text-align": "center",
                      padding: "60px 20px",
                      color: "#999",
                    }}
                  >
                    <div
                      style={{ "font-size": "48px", "margin-bottom": "10px" }}
                    >
                      📭
                    </div>
                    <div>
                      {searchQuery()
                        ? "没有找到匹配的密码"
                        : "还没有保存的密码"}
                    </div>
                  </div>
                </Show>
                <Show when={filteredEntries().length > 0}>
                  <For each={filteredEntries()}>
                    {(entry) => (
                      <div
                        onClick={(e) => {
                          e.preventDefault();
                          e.stopPropagation();
                          handleSelectEntry(entry);
                        }}
                        style={{
                          padding: "12px 15px",
                          margin: "0 0 8px 0",
                          background:
                            selectedEntry()?.id === entry.id
                              ? "#0078d4"
                              : "white",
                          color:
                            selectedEntry()?.id === entry.id ? "white" : "#333",
                          "border-radius": "3px",
                          cursor: "pointer",
                          "user-select": "none",
                          border:
                            selectedEntry()?.id === entry.id
                              ? "1px solid #0078d4"
                              : "1px solid #e0e0e0",
                          transition: "all 0.15s ease",
                        }}
                        onMouseEnter={(e) => {
                          if (selectedEntry()?.id !== entry.id) {
                            e.currentTarget.style.background = "#fafafa";
                          }
                        }}
                        onMouseLeave={(e) => {
                          if (selectedEntry()?.id !== entry.id) {
                            e.currentTarget.style.background = "white";
                          }
                        }}
                      >
                        <div
                          style={{
                            "font-weight": "600",
                            "margin-bottom": "4px",
                          }}
                        >
                          {entry.service}
                        </div>
                        <div style={{ "font-size": "13px", opacity: 0.8 }}>
                          {entry.username}
                        </div>
                      </div>
                    )}
                  </For>
                </Show>
              </div>

              {/* 底部统计 */}
              <div
                style={{
                  padding: "10px 15px",
                  background: "#fafafa",
                  "border-top": "1px solid #e0e0e0",
                  "font-size": "12px",
                  color: "#666",
                }}
              >
                共 {passwordEntries().length} 个密码
                <Show when={searchQuery() !== ""}>
                  <span> / 显示 {filteredEntries().length} 个结果</span>
                </Show>
              </div>
            </div>

            {/* 右侧:表单详情 */}
            <div
              style={{
                flex: 1,
                background: "white",
                "border-radius": "6px",
                border: "1px solid #e0e0e0",
                overflow: "auto",
                "box-shadow": "0 1px 3px rgba(0,0,0,0.08)",
              }}
            >
              <div style={{ padding: "20px" }}>
                <h2
                  style={{
                    margin: "0 0 20px 0",
                    color: "#1e1e1e",
                    "font-size": "18px",
                    "font-weight": "600",
                    "border-bottom": "2px solid #0078d4",
                    "padding-bottom": "8px",
                  }}
                >
                  {isEditMode() ? "编辑密码" : "新建密码"}
                </h2>

                <Show when={selectedEntry()}>
                  <div
                    style={{
                      margin: "0 0 20px 0",
                      padding: "12px",
                      background: "#fff8e1",
                      "border-left": "4px solid #ffc107",
                      "border-radius": "3px",
                    }}
                  >
                    <div
                      style={{
                        display: "flex",
                        "justify-content": "space-between",
                        "align-items": "center",
                      }}
                    >
                      <span style={{ color: "#856404", "font-size": "14px" }}>
                        正在编辑密码
                      </span>
                      <button
                        onClick={(e) => {
                          e.preventDefault();
                          e.stopPropagation();
                          handleDeletePassword(selectedEntry()!.id);
                        }}
                        style={{
                          padding: "6px 12px",
                          background: "#d13438",
                          color: "white",
                          border: "none",
                          "border-radius": "3px",
                          cursor: "pointer",
                          "font-size": "12px",
                          "font-weight": "500",
                        }}
                      >
                        🗑️ 删除
                      </button>
                    </div>
                  </div>
                </Show>

                <For each={pluginView()!.fields}>
                  {(field) => (
                    <div style={{ "margin-bottom": "20px" }}>
                      <Show when={field.type === "input"}>
                        <div>
                          <label
                            style={{
                              display: "block",
                              "margin-bottom": "6px",
                              "font-weight": "500",
                              color: "#1e1e1e",
                              "font-size": "13px",
                            }}
                          >
                            {field.label}
                          </label>
                          <input
                            type={field.inputType || "text"}
                            placeholder={field.placeholder}
                            value={formData()[field.key] || ""}
                            style={{
                              width: "100%",
                              padding: "8px 10px",
                              border: formErrors()[field.key]
                                ? "2px solid #d13438"
                                : "1px solid #d0d0d0",
                              "border-radius": "3px",
                              "font-size": "13px",
                              "font-family": "inherit",
                              transition: "border-color 0.15s",
                            }}
                            onInput={(e) =>
                              handleFieldChange(
                                field.key,
                                e.currentTarget.value,
                                field,
                              )
                            }
                          />
                          <Show when={formErrors()[field.key]}>
                            <div
                              style={{
                                "margin-top": "4px",
                                color: "#d13438",
                                "font-size": "12px",
                              }}
                            >
                              {formErrors()[field.key]}
                            </div>
                          </Show>
                        </div>
                      </Show>
                      <Show when={field.type === "button"}>
                        <button
                          onClick={(e) => {
                            e.preventDefault();
                            e.stopPropagation();
                            handleAction(field.key);
                          }}
                          disabled={!isFormValid()}
                          style={{
                            padding: "10px 20px",
                            background: isFormValid() ? "#0078d4" : "#a0a0a0",
                            color: "white",
                            border: "none",
                            "border-radius": "3px",
                            "font-weight": "500",
                            cursor: isFormValid() ? "pointer" : "not-allowed",
                            "font-size": "14px",
                            transition: "all 0.15s",
                            opacity: isFormValid() ? 1 : 0.6,
                            width: "100%",
                          }}
                        >
                          {isEditMode() ? "💾 更新密码" : field.label}
                        </button>
                      </Show>
                    </div>
                  )}
                </For>

                <Show when={selectedEntry()}>
                  <div
                    style={{
                      "margin-top": "20px",
                      padding: "12px",
                      background: "#fafafa",
                      "border-radius": "3px",
                      border: "1px solid #e0e0e0",
                    }}
                  >
                    <div
                      style={{
                        "font-size": "11px",
                        color: "#666",
                        "margin-bottom": "4px",
                        "text-transform": "uppercase",
                        "letter-spacing": "0.5px",
                      }}
                    >
                      创建时间
                    </div>
                    <div style={{ "font-size": "13px", color: "#333" }}>
                      {new Date(selectedEntry()!.created_at).toLocaleString()}
                    </div>
                  </div>
                </Show>
              </div>
            </div>
          </div>
        </Show>

        <Show
          when={selectedPlugin() && selectedPlugin() !== "password-manager"}
        >
          <div
            style={{
              padding: "40px",
              "text-align": "center",
              color: "#7f8c8d",
            }}
          >
            <div style={{ "font-size": "64px", "margin-bottom": "20px" }}>
              🚧
            </div>
            <h2 style={{ "font-size": "24px", margin: "0 0 10px 0" }}>
              插件开发中
            </h2>
            <p>该插件正在开发中,敬请期待...</p>
          </div>
        </Show>
      </div>

      {/* 设置对话框 */}
      <Show when={showSettings()}>
        <div
          style={{
            position: "fixed",
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: "rgba(0,0,0,0.5)",
            display: "flex",
            "align-items": "center",
            "justify-content": "center",
            "z-index": 1000,
          }}
        >
          <div
            style={{
              background: "white",
              "border-radius": "8px",
              width: "500px",
              "max-height": "400px",
              "box-shadow": "0 4px 20px rgba(0,0,0,0.3)",
            }}
          >
            <div
              style={{
                padding: "20px",
                "border-bottom": "1px solid #dee2e6",
                display: "flex",
                "justify-content": "space-between",
                "align-items": "center",
              }}
            >
              <h3 style={{ margin: 0 }}>设置</h3>
              <button
                onClick={() => setShowSettings(false)}
                style={{
                  background: "transparent",
                  border: "none",
                  "font-size": "20px",
                  cursor: "pointer",
                  color: "#999",
                }}
              >
                ✕
              </button>
            </div>
            <div style={{ padding: "20px" }}>
              <h4 style={{ margin: "0 0 15px 0", color: "#2c3e50" }}>外观</h4>
              <div style={{ "margin-bottom": "20px" }}>
                <label
                  style={{
                    display: "block",
                    "margin-bottom": "5px",
                    "font-weight": "600",
                  }}
                >
                  主题
                </label>
                <select
                  value={theme()}
                  onChange={(e) => setTheme(e.currentTarget.value)}
                  style={{
                    width: "100%",
                    padding: "8px",
                    border: "1px solid #ced4da",
                    "border-radius": "4px",
                  }}
                >
                  <option value="light">浅色</option>
                  <option value="dark">深色</option>
                </select>
              </div>

              <h4 style={{ margin: "0 0 15px 0", color: "#2c3e50" }}>通用</h4>
              <div style={{ "margin-bottom": "15px" }}>
                <label
                  style={{
                    display: "flex",
                    "align-items": "center",
                    gap: "10px",
                    cursor: "pointer",
                  }}
                >
                  <input
                    type="checkbox"
                    checked={autoStart()}
                    onChange={(e) => setAutoStart(e.currentTarget.checked)}
                    style={{ width: "18px", height: "18px" }}
                  />
                  开机自动启动
                </label>
              </div>
              <div style={{ "margin-bottom": "20px" }}>
                <label
                  style={{
                    display: "flex",
                    "align-items": "center",
                    gap: "10px",
                    cursor: "pointer",
                  }}
                >
                  <input
                    type="checkbox"
                    checked={minimizeToTray()}
                    onChange={(e) => setMinimizeToTray(e.currentTarget.checked)}
                    style={{ width: "18px", height: "18px" }}
                  />
                  最小化到系统托盘
                </label>
              </div>
            </div>
            <div
              style={{
                padding: "15px 20px",
                "border-top": "1px solid #dee2e6",
                "text-align": "right",
              }}
            >
              <button
                onClick={handleSaveSettings}
                style={{
                  padding: "8px 20px",
                  background: "#3498db",
                  color: "white",
                  border: "none",
                  "border-radius": "4px",
                  cursor: "pointer",
                }}
              >
                保存
              </button>
            </div>
          </div>
        </div>
      </Show>

      {/* 日志对话框 */}
      <Show when={showLogs()}>
        <div
          style={{
            position: "fixed",
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: "rgba(0,0,0,0.5)",
            display: "flex",
            "align-items": "center",
            "justify-content": "center",
            "z-index": 1000,
          }}
        >
          <div
            style={{
              background: "white",
              "border-radius": "8px",
              width: "800px",
              height: "600px",
              "box-shadow": "0 4px 20px rgba(0,0,0,0.3)",
              display: "flex",
              "flex-direction": "column",
            }}
          >
            <div
              style={{
                padding: "20px",
                "border-bottom": "1px solid #dee2e6",
                display: "flex",
                "justify-content": "space-between",
                "align-items": "center",
              }}
            >
              <h3 style={{ margin: 0 }}>系统日志</h3>
              <button
                onClick={() => setShowLogs(false)}
                style={{
                  background: "transparent",
                  border: "none",
                  "font-size": "20px",
                  cursor: "pointer",
                  color: "#999",
                }}
              >
                ✕
              </button>
            </div>
            <div
              style={{
                flex: 1,
                padding: "20px",
                overflow: "auto",
                background: "#1e1e1e",
                color: "#d4d4d4",
                "font-family": "monospace",
                "font-size": "13px",
                "line-height": "1.6",
              }}
            >
              <div>[INFO] Work Tools 应用启动成功</div>
              <div>[INFO] 插件管理器初始化完成</div>
              <div>[INFO] 发现 {plugins().length} 个已安装插件</div>
              <div>[INFO] 密码管理器加载成功</div>
            </div>
          </div>
        </div>
      </Show>

      {/* 插件市场对话框 */}
      <Show when={showPluginMarket()}>
        <div
          style={{
            position: "fixed",
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: "rgba(0,0,0,0.5)",
            display: "flex",
            "align-items": "center",
            "justify-content": "center",
            "z-index": 1000,
          }}
        >
          <div
            style={{
              background: "white",
              "border-radius": "8px",
              width: "600px",
              height: "400px",
              "box-shadow": "0 4px 20px rgba(0,0,0,0.3)",
              display: "flex",
              "flex-direction": "column",
            }}
          >
            <div
              style={{
                padding: "20px",
                "border-bottom": "1px solid #dee2e6",
                display: "flex",
                "justify-content": "space-between",
                "align-items": "center",
              }}
            >
              <h3 style={{ margin: 0 }}>插件市场</h3>
              <button
                onClick={() => setShowPluginMarket(false)}
                style={{
                  background: "transparent",
                  border: "none",
                  "font-size": "20px",
                  cursor: "pointer",
                  color: "#999",
                }}
              >
                ✕
              </button>
            </div>
            <div
              style={{
                flex: 1,
                padding: "20px",
                overflow: "auto",
              }}
            >
              <For each={plugins()}>
                {(plugin) => (
                  <div
                    style={{
                      padding: "15px",
                      margin: "0 0 10px 0",
                      background: "#f8f9fa",
                      "border-radius": "4px",
                      border: "1px solid #dee2e6",
                    }}
                  >
                    <div
                      style={{
                        display: "flex",
                        "align-items": "center",
                        gap: "15px",
                      }}
                    >
                      <span style={{ "font-size": "32px" }}>{plugin.icon}</span>
                      <div style={{ flex: 1 }}>
                        <div
                          style={{
                            "font-weight": "600",
                            "margin-bottom": "5px",
                          }}
                        >
                          {plugin.name}
                        </div>
                        <div style={{ "font-size": "13px", color: "#666" }}>
                          {plugin.description}
                        </div>
                        <div
                          style={{
                            "font-size": "12px",
                            color: "#999",
                            "margin-top": "5px",
                          }}
                        >
                          版本: {plugin.version}
                        </div>
                      </div>
                      <div
                        style={{
                          padding: "4px 12px",
                          background: "#27ae60",
                          color: "white",
                          "border-radius": "4px",
                          "font-size": "12px",
                        }}
                      >
                        已安装
                      </div>
                    </div>
                  </div>
                )}
              </For>
            </div>
          </div>
        </div>
      </Show>

      {/* 诊断对话框 */}
      <Show when={showDiagnostics()}>
        <div
          style={{
            position: "fixed",
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: "rgba(0,0,0,0.5)",
            display: "flex",
            "align-items": "center",
            "justify-content": "center",
            "z-index": 1000,
          }}
        >
          <div
            style={{
              background: "white",
              "border-radius": "8px",
              width: "700px",
              height: "500px",
              "box-shadow": "0 4px 20px rgba(0,0,0,0.3)",
              display: "flex",
              "flex-direction": "column",
            }}
          >
            <div
              style={{
                padding: "20px",
                "border-bottom": "1px solid #dee2e6",
                display: "flex",
                "justify-content": "space-between",
                "align-items": "center",
              }}
            >
              <h3 style={{ margin: 0 }}>诊断信息</h3>
              <button
                onClick={() => setShowDiagnostics(false)}
                style={{
                  background: "transparent",
                  border: "none",
                  "font-size": "20px",
                  cursor: "pointer",
                  color: "#999",
                }}
              >
                ✕
              </button>
            </div>
            <div
              style={{
                flex: 1,
                padding: "20px",
                overflow: "auto",
                background: "#1e1e1e",
                color: "#d4d4d4",
                "font-family": "monospace",
                "font-size": "12px",
                "line-height": "1.6",
                "white-space": "pre-wrap",
              }}
            >
              {diagnostics().join("\n")}
            </div>
            <div
              style={{
                padding: "15px 20px",
                "border-top": "1px solid #dee2e6",
                "text-align": "right",
              }}
            >
              <button
                onClick={runDiagnostics}
                style={{
                  padding: "8px 20px",
                  background: "#0078d4",
                  color: "white",
                  border: "none",
                  "border-radius": "3px",
                  cursor: "pointer",
                  "font-size": "13px",
                }}
              >
                重新运行
              </button>
            </div>
          </div>
        </div>
      </Show>
    </div>
  );
}

export default App;
