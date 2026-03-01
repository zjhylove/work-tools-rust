import { For, Show, createSignal, onMount, createMemo } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { openUrl as openUrlTauri } from "@tauri-apps/plugin-opener";
import "./App.css";

// 安全的 invoke 包装函数 - Tauri 2.x 的 invoke 函数会自动处理环境检测
const safeInvoke = async <T,>(command: string, args?: any): Promise<T> => {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    console.error("Invoke error:", error);
    throw error;
  }
};

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
  // 密码可见性状态: key 是 entry.id, value 是布尔值表示密码是否可见
  const [visiblePasswords, setVisiblePasswords] = createSignal<
    Record<string, boolean>
  >({});

  // 视图模式: 'list' 显示密码列表, 'form' 显示添加/编辑表单
  const [viewMode, setViewMode] = createSignal<"list" | "form">("list");

  // 对话框状态
  const [showSettings, setShowSettings] = createSignal(false);
  const [showLogs, setShowLogs] = createSignal(false);
  const [showPluginMarket, setShowPluginMarket] = createSignal(false);
  const [showDiagnostics, setShowDiagnostics] = createSignal(false);
  const [diagnostics, setDiagnostics] = createSignal<string[]>([]);
  const [theme, setTheme] = createSignal("light");
  const [autoStart, setAutoStart] = createSignal(false);
  const [minimizeToTray, setMinimizeToTray] = createSignal(true);

  // 主密码验证状态
  const [showMasterPasswordDialog, setShowMasterPasswordDialog] =
    createSignal(false);
  const [masterPassword, setMasterPassword] = createSignal("");
  const [isFirstTimeSetup, setIsFirstTimeSetup] = createSignal(false);
  const [masterPasswordError, setMasterPasswordError] = createSignal("");
  const [isAuthenticated, setIsAuthenticated] = createSignal(false);

  // 删除确认对话框状态
  const [showDeleteConfirm, setShowDeleteConfirm] = createSignal(false);
  const [entryToDelete, setEntryToDelete] = createSignal<string | null>(null);

  // Toast 提示状态
  const [toastMessage, setToastMessage] = createSignal("");
  const [showToast, setShowToast] = createSignal(false);

  // 加载插件列表
  onMount(async () => {
    console.log("=== App onMount 开始 ===");
    console.log("当前时间:", new Date().toISOString());

    // 检查是否在 Tauri 环境中
    const tauriAvailable =
      typeof window !== "undefined" && "__TAURI__" in window;
    console.log("Tauri 环境检查:", tauriAvailable);
    console.log("window.__TAURI__ 存在:", "__TAURI__" in window);
    console.log(
      "window.__TAURI__.core 存在:",
      (window as any).__TAURI__?.core ? "true" : "false",
    );

    // 检查 CSS 变量是否加载
    const rootElement = document.documentElement;
    const computedStyle = getComputedStyle(rootElement);
    console.log("=== CSS 变量诊断 ===");
    console.log(
      "--bg-secondary:",
      computedStyle.getPropertyValue("--bg-secondary"),
    );
    console.log(
      "--bg-primary:",
      computedStyle.getPropertyValue("--bg-primary"),
    );
    console.log("--accent:", computedStyle.getPropertyValue("--accent"));

    // 如果不在 Tauri 环境,使用模拟数据
    if (!tauriAvailable) {
      console.warn("⚠️ 不在 Tauri 环境,使用模拟数据");
      const mockPlugins: PluginInfo[] = [
        {
          id: "password-manager",
          name: "密码管理器",
          description: "本地安全存储和管理密码",
          version: "1.0.0",
          icon: "🔐",
        },
        {
          id: "auth",
          name: "双因素验证",
          description: "TOTP 双因素认证",
          version: "1.0.0",
          icon: "🔐",
        },
      ];
      setPlugins(mockPlugins);
      setSelectedPlugin(mockPlugins[0].id);
      setLoading(false);
      return;
    }

    try {
      console.log("调用 get_installed_plugins...");
      const installedPlugins = await safeInvoke<PluginInfo[]>(
        "get_installed_plugins",
      );
      console.log("已安装插件 (原始):", installedPlugins);
      console.log("已安装插件数量:", installedPlugins.length);

      if (Array.isArray(installedPlugins)) {
        console.log("✅ 插件数据是数组，数量:", installedPlugins.length);
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
          const config = await safeInvoke<any>("get_app_config");
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

  // 检查主密码状态
  const checkMasterPasswordStatus = async () => {
    try {
      const hasPassword = await safeInvoke<boolean>("has_master_password");
      if (!hasPassword) {
        setIsFirstTimeSetup(true);
      }
      return hasPassword;
    } catch (error) {
      console.error("检查主密码状态失败:", error);
      return false;
    }
  };

  // 显示主密码验证对话框
  const showMasterPasswordPrompt = async () => {
    const hasPassword = await checkMasterPasswordStatus();
    setIsFirstTimeSetup(!hasPassword);
    setShowMasterPasswordDialog(true);
  };

  // 验证主密码
  const verifyMasterPassword = async () => {
    try {
      const password = masterPassword();
      if (!password || password.length < 6) {
        setMasterPasswordError("主密码至少需要 6 个字符");
        return false;
      }

      const result = await safeInvoke<boolean>(
        "init_or_verify_master_password",
        {
          password,
        },
      );

      if (result) {
        setIsAuthenticated(true);
        setShowMasterPasswordDialog(false);
        setMasterPassword("");
        setMasterPasswordError("");

        // 验证成功后,加载密码列表
        if (selectedPlugin() === "password-manager") {
          const entries = await safeInvoke<PasswordEntry[]>(
            "get_password_entries",
          );
          setPasswordEntries(entries);
        }

        return true;
      } else {
        setMasterPasswordError(isFirstTimeSetup() ? "设置失败" : "密码错误");
        return false;
      }
    } catch (error) {
      console.error("主密码验证失败:", error);
      setMasterPasswordError("验证失败: " + error);
      return false;
    }
  };

  const openPlugin = async (pluginId: string) => {
    console.log("打开插件:", pluginId);
    setSelectedPlugin(pluginId);

    // 先设置 UI Schema (无论是否验证主密码都需要)
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
          inputType: "password",
          placeholder: "输入密码",
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

    if (pluginId === "password-manager") {
      // 检查是否已通过主密码验证
      if (!isAuthenticated()) {
        await showMasterPasswordPrompt();
        return;
      }

      try {
        const entries = await safeInvoke<PasswordEntry[]>(
          "get_password_entries",
        );
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
        const entries = await safeInvoke<any[]>("get_auth_entries");
        console.log("加载到的认证条目:", entries);
      } catch (error) {
        console.error("加载认证列表失败:", error);
      }
    }
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

  // 使用 createMemo 创建响应式的表单验证状态
  const isFormValid = createMemo(() => {
    const fields = pluginView()?.fields || [];

    for (const field of fields) {
      if (field.type === "input" && field.required) {
        const value = formData()[field.key] || "";
        console.log(
          `检查字段 ${field.key}, value: "${value}", trim: "${value.trim()}"`,
        );
        if (!value.trim()) {
          console.log(`字段 ${field.key} 为空,返回 false`);
          return false;
        }
        if (field.minLength && value.length < field.minLength) {
          console.log(`字段 ${field.key} 长度不足,返回 false`);
          return false;
        }
        if (field.pattern) {
          const regex = new RegExp(field.pattern);
          if (!regex.test(value)) {
            console.log(`字段 ${field.key} 格式不匹配,返回 false`);
            return false;
          }
        }
      }
    }
    console.log("表单验证通过,返回 true");
    return true;
  });

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
      // 检查是否已通过主密码验证
      if (!isAuthenticated()) {
        console.log("未验证主密码,显示提示");
        alert("请先验证主密码");
        await showMasterPasswordPrompt();
        return;
      }

      if (!validateForm()) {
        console.log("表单验证失败");
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
        await safeInvoke("save_password_entry", { entry });
        const entries = await safeInvoke<PasswordEntry[]>(
          "get_password_entries",
        );
        setPasswordEntries(entries);
        alert(isEditMode() ? "密码更新成功!" : "密码保存成功!");
        setFormData({});
        setFormErrors({});
        setSelectedEntry(null);
        setIsEditMode(false);
        setViewMode("list"); // 返回到列表视图
      } catch (error) {
        console.error("保存密码失败:", error);
        alert("保存密码失败: " + error);
      }
    }
  };

  const handleSelectEntry = async (entry: PasswordEntry) => {
    // 检查是否已通过主密码验证
    if (!isAuthenticated()) {
      await showMasterPasswordPrompt();
      return;
    }

    setSelectedEntry(entry);
    setIsEditMode(true);
    setFormData({
      url: entry.url || "",
      service: entry.service,
      username: entry.username,
      password: entry.password,
    });
    setFormErrors({});
    setViewMode("form"); // 切换到表单视图进行编辑
  };

  const handleAddNew = async () => {
    // 检查是否已通过主密码验证
    if (!isAuthenticated()) {
      await showMasterPasswordPrompt();
      return;
    }

    setSelectedEntry(null);
    setIsEditMode(false);
    setFormData({});
    setFormErrors({});
    setViewMode("form"); // 切换到表单视图
  };

  const handleDeletePassword = async (id: string) => {
    // 检查是否已通过主密码验证
    if (!isAuthenticated()) {
      await showMasterPasswordPrompt();
      return;
    }

    // 使用自定义确认对话框
    setEntryToDelete(id);
    setShowDeleteConfirm(true);
  };

  const confirmDeletePassword = async () => {
    const id = entryToDelete();
    if (!id) return;

    try {
      await safeInvoke("delete_password_entry", { id });
      const entries = await safeInvoke<PasswordEntry[]>("get_password_entries");
      setPasswordEntries(entries);

      // 删除后返回列表视图
      setSelectedEntry(null);
      setIsEditMode(false);
      setFormData({});
      setFormErrors({});
      setViewMode("list");

      // 关闭确认对话框
      setShowDeleteConfirm(false);
      setEntryToDelete(null);

      alert("删除成功!");
    } catch (error) {
      console.error("删除密码失败:", error);
      alert("删除失败: " + error);
    }
  };

  const cancelDeletePassword = () => {
    setShowDeleteConfirm(false);
    setEntryToDelete(null);
  };

  // 切换密码可见性
  const togglePasswordVisibility = (entryId: string) => {
    setVisiblePasswords((prev) => ({
      ...prev,
      [entryId]: !prev[entryId],
    }));
  };

  // 显示 Toast 提示
  const showToastMessage = (message: string) => {
    setToastMessage(message);
    setShowToast(true);
    setTimeout(() => {
      setShowToast(false);
    }, 2000); // 2秒后自动消失
  };

  // 复制密码到剪贴板
  const copyPassword = async (password: string) => {
    try {
      await navigator.clipboard.writeText(password);
      showToastMessage("✓ 密码已复制!");
    } catch (error) {
      console.error("复制失败:", error);
      showToastMessage("✗ 复制失败");
    }
  };

  // 打开 URL 链接
  const openUrl = async (url: string) => {
    if (!url) return;
    try {
      await openUrlTauri(url);
      showToastMessage("✓ 已打开链接!");
    } catch (error) {
      console.error("打开链接失败:", error);
      showToastMessage("✗ 打开链接失败");
    }
  };

  const handleSaveSettings = async () => {
    try {
      await safeInvoke("set_app_config", {
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
      const installed = await safeInvoke<PluginInfo[]>("get_installed_plugins");
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
      const available = await safeInvoke<PluginInfo[]>("get_available_plugins");
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
        class="sidebar-container"
        style={{
          width: "250px",
          display: "flex",
          "flex-direction": "column",
          "flex-shrink": 0,
        }}
      >
        {/* 标题 */}
        <div
          style={{
            padding: "20px 15px",
            "text-align": "center",
            "border-bottom": "1px solid var(--border-color)",
          }}
        >
          <h2
            style={{
              margin: 0,
              "font-size": "18px",
              "font-weight": "600",
              color: "var(--text-primary)",
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
                        ? "var(--accent-light)"
                        : "transparent",
                    "border-left":
                      selectedPlugin() === plugin.id
                        ? "3px solid var(--accent)"
                        : "3px solid transparent",
                    transition: "all 0.15s ease",
                    color:
                      selectedPlugin() === plugin.id
                        ? "var(--accent)"
                        : "var(--text-primary)",
                  }}
                  onMouseEnter={(e) => {
                    if (selectedPlugin() !== plugin.id) {
                      e.currentTarget.style.background = "var(--hover-bg)";
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
                          color: "var(--text-secondary)",
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
            "border-top": "1px solid var(--border-color)",
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
              color: "var(--text-secondary)",
              cursor: "pointer",
              "border-radius": "4px",
              "font-size": "18px",
              transition: "background 0.2s",
            }}
            onMouseEnter={(e) =>
              (e.currentTarget.style.background = "var(--hover-bg)")
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
              color: "var(--text-secondary)",
              cursor: "pointer",
              "border-radius": "4px",
              "font-size": "18px",
              transition: "background 0.2s",
            }}
            onMouseEnter={(e) =>
              (e.currentTarget.style.background = "var(--hover-bg)")
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
              color: "var(--text-secondary)",
              cursor: "pointer",
              "border-radius": "4px",
              "font-size": "18px",
              transition: "background 0.2s",
            }}
            onMouseEnter={(e) =>
              (e.currentTarget.style.background = "var(--hover-bg)")
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
              color: "var(--text-secondary)",
              cursor: "pointer",
              "border-radius": "4px",
              "font-size": "18px",
              transition: "background 0.2s",
            }}
            onMouseEnter={(e) =>
              (e.currentTarget.style.background = "var(--hover-bg)")
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
              padding: "20px",
              height: "100%",
              "box-sizing": "border-box",
              display: "flex",
              "flex-direction": "column",
            }}
          >
            {/* 列表视图 */}
            <Show when={viewMode() === "list"}>
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
                          style={{
                            padding: "12px 15px",
                            margin: "0 0 8px 0",
                            background: "white",
                            color: "#333",
                            "border-radius": "3px",
                            "user-select": "none",
                            border: "1px solid #e0e0e0",
                            transition: "all 0.15s ease",
                            display: "flex",
                            "align-items": "center",
                            gap: "12px",
                          }}
                          onMouseEnter={(e) => {
                            e.currentTarget.style.background = "#fafafa";
                          }}
                          onMouseLeave={(e) => {
                            e.currentTarget.style.background = "white";
                          }}
                        >
                          <div style={{ flex: 1, "min-width": 0 }}>
                            <div
                              style={{
                                "font-weight": "600",
                                "margin-bottom": "4px",
                                overflow: "hidden",
                                "text-overflow": "ellipsis",
                                "white-space": "nowrap",
                              }}
                            >
                              {entry.service}
                            </div>
                            <div
                              style={{
                                "font-size": "13px",
                                opacity: 0.8,
                                overflow: "hidden",
                                "text-overflow": "ellipsis",
                                "white-space": "nowrap",
                              }}
                            >
                              {entry.username}
                            </div>
                            <div
                              style={{
                                "font-size": "12px",
                                opacity: 0.7,
                                "margin-top": "4px",
                                "font-family": "monospace",
                                overflow: "hidden",
                                "text-overflow": "ellipsis",
                                "white-space": "nowrap",
                              }}
                            >
                              {visiblePasswords()[entry.id]
                                ? entry.password
                                : "••••••••"}
                            </div>
                          </div>
                          <div
                            style={{
                              display: "flex",
                              gap: "6px",
                              "flex-shrink": 0,
                            }}
                          >
                            {/* 显示/隐藏密码按钮 */}
                            <button
                              onClick={(e) => {
                                e.preventDefault();
                                e.stopPropagation();
                                togglePasswordVisibility(entry.id);
                              }}
                              title={
                                visiblePasswords()[entry.id]
                                  ? "隐藏密码"
                                  : "显示密码"
                              }
                              style={{
                                padding: "6px 10px",
                                background: "#6c757d",
                                color: "white",
                                border: "none",
                                "border-radius": "3px",
                                cursor: "pointer",
                                "font-size": "12px",
                                "font-weight": "500",
                                transition: "background 0.15s",
                              }}
                              onMouseEnter={(e) =>
                                (e.currentTarget.style.background = "#5a6268")
                              }
                              onMouseLeave={(e) =>
                                (e.currentTarget.style.background = "#6c757d")
                              }
                            >
                              {visiblePasswords()[entry.id] ? "🙈" : "👁️"}
                            </button>
                            {/* 复制密码按钮 */}
                            <button
                              onClick={(e) => {
                                e.preventDefault();
                                e.stopPropagation();
                                copyPassword(entry.password);
                              }}
                              title="复制密码"
                              style={{
                                padding: "6px 10px",
                                background: "#28a745",
                                color: "white",
                                border: "none",
                                "border-radius": "3px",
                                cursor: "pointer",
                                "font-size": "12px",
                                "font-weight": "500",
                                transition: "background 0.15s",
                              }}
                              onMouseEnter={(e) =>
                                (e.currentTarget.style.background = "#218838")
                              }
                              onMouseLeave={(e) =>
                                (e.currentTarget.style.background = "#28a745")
                              }
                            >
                              📋
                            </button>
                            {/* 打开链接按钮 (仅当有 URL 时显示) */}
                            <Show when={entry.url}>
                              <button
                                onClick={(e) => {
                                  e.preventDefault();
                                  e.stopPropagation();
                                  openUrl(entry.url!);
                                }}
                                title="打开链接"
                                style={{
                                  padding: "6px 10px",
                                  background: "#17a2b8",
                                  color: "white",
                                  border: "none",
                                  "border-radius": "3px",
                                  cursor: "pointer",
                                  "font-size": "12px",
                                  "font-weight": "500",
                                  transition: "background 0.15s",
                                }}
                                onMouseEnter={(e) =>
                                  (e.currentTarget.style.background = "#138496")
                                }
                                onMouseLeave={(e) =>
                                  (e.currentTarget.style.background = "#17a2b8")
                                }
                              >
                                🔗
                              </button>
                            </Show>
                            <button
                              onClick={(e) => {
                                e.preventDefault();
                                e.stopPropagation();
                                handleSelectEntry(entry);
                              }}
                              title="编辑"
                              style={{
                                padding: "6px 10px",
                                background: "#0078d4",
                                color: "white",
                                border: "none",
                                "border-radius": "3px",
                                cursor: "pointer",
                                "font-size": "12px",
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
                              ✏️ 编辑
                            </button>
                            <button
                              onClick={(e) => {
                                console.log(
                                  "删除按钮被点击, entry.id:",
                                  entry.id,
                                );
                                e.preventDefault();
                                e.stopPropagation();
                                handleDeletePassword(entry.id);
                              }}
                              title="删除"
                              style={{
                                padding: "6px 10px",
                                background: "#d13438",
                                color: "white",
                                border: "none",
                                "border-radius": "3px",
                                cursor: "pointer",
                                "font-size": "12px",
                                "font-weight": "500",
                                transition: "background 0.15s",
                              }}
                              onMouseEnter={(e) =>
                                (e.currentTarget.style.background = "#a52a2a")
                              }
                              onMouseLeave={(e) =>
                                (e.currentTarget.style.background = "#d13438")
                              }
                            >
                              🗑️ 删除
                            </button>
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
            </Show>

            {/* 表单视图 */}
            <Show when={viewMode() === "form"}>
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
                  {/* 标题栏 */}
                  <div
                    style={{
                      display: "flex",
                      "justify-content": "space-between",
                      "align-items": "center",
                      "margin-bottom": "20px",
                      "padding-bottom": "10px",
                      "border-bottom": "2px solid #0078d4",
                    }}
                  >
                    <h2
                      style={{
                        margin: 0,
                        color: "#1e1e1e",
                        "font-size": "18px",
                        "font-weight": "600",
                      }}
                    >
                      {isEditMode() ? "编辑密码" : "新建密码"}
                    </h2>
                    <button
                      onClick={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        setViewMode("list");
                        setSelectedEntry(null);
                        setIsEditMode(false);
                        setFormData({});
                        setFormErrors({});
                      }}
                      style={{
                        padding: "6px 12px",
                        background: "#6c757d",
                        color: "white",
                        border: "none",
                        "border-radius": "3px",
                        cursor: "pointer",
                        "font-size": "12px",
                        "font-weight": "500",
                      }}
                    >
                      ✕ 返回列表
                    </button>
                  </div>

                  <Show when={pluginView()}>
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
                                console.log(
                                  "按钮被点击, field.key:",
                                  field.key,
                                );
                                e.preventDefault();
                                e.stopPropagation();
                                handleAction(field.key);
                              }}
                              disabled={!isFormValid()}
                              style={{
                                padding: "10px 20px",
                                background: isFormValid()
                                  ? "#0078d4"
                                  : "#a0a0a0",
                                color: "white",
                                border: "none",
                                "border-radius": "3px",
                                "font-weight": "500",
                                cursor: isFormValid()
                                  ? "pointer"
                                  : "not-allowed",
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
                  </Show>

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
            </Show>
          </div>
        </Show>

        {/* Auth Plugin 界面 */}
        <Show when={selectedPlugin() === "auth"}>
          <div
            style={{
              padding: "40px",
              "text-align": "center",
              color: "#666",
            }}
          >
            <div style={{ "font-size": "64px", "margin-bottom": "20px" }}>
              🔐
            </div>
            <h2
              style={{
                "font-size": "24px",
                margin: "0 0 10px 0",
                color: "#1e1e1e",
              }}
            >
              双因素验证
            </h2>
            <p>该插件正在开发中,敬请期待...</p>
            <div
              style={{
                "margin-top": "20px",
                padding: "15px",
                background: "#fff3cd",
                "border-radius": "4px",
                "border-left": "4px solid #ffc107",
                "text-align": "left",
                "max-width": "600px",
                "margin-left": "auto",
                "margin-right": "auto",
              }}
            >
              <div style={{ "font-weight": "600", "margin-bottom": "8px" }}>
                功能特性:
              </div>
              <ul style={{ margin: 0, "padding-left": "20px" }}>
                <li>支持 TOTP (Time-based One-Time Password)</li>
                <li>兼容 Google Authenticator、Authy 等</li>
                <li>自动更新验证码</li>
                <li>点击复制功能</li>
                <li>QR 码导入</li>
              </ul>
            </div>
          </div>
        </Show>

        {/* 其他插件提示 */}
        <Show
          when={
            selectedPlugin() &&
            selectedPlugin() !== "password-manager" &&
            selectedPlugin() !== "auth"
          }
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

      {/* 主密码验证对话框 */}
      <Show when={showMasterPasswordDialog()}>
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
              width: "400px",
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
              <h2
                style={{
                  margin: 0,
                  "font-size": "18px",
                  "font-weight": "600",
                  color: "#333",
                }}
              >
                {isFirstTimeSetup() ? "设置主密码" : "输入主密码"}
              </h2>
            </div>

            <div style={{ padding: "20px" }}>
              <p
                style={{
                  margin: "0 0 15px 0",
                  "font-size": "14px",
                  color: "#666",
                  "line-height": "1.5",
                }}
              >
                {isFirstTimeSetup()
                  ? "首次使用需要设置主密码,主密码将用于加密所有密码数据。"
                  : "请输入主密码以访问密码管理器。"}
              </p>

              <div style={{ "margin-bottom": "15px" }}>
                <input
                  type="password"
                  placeholder="请输入主密码 (至少 6 个字符)"
                  value={masterPassword()}
                  onInput={(e) => {
                    setMasterPassword(e.currentTarget.value);
                    setMasterPasswordError("");
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      verifyMasterPassword();
                    }
                  }}
                  style={{
                    width: "100%",
                    padding: "10px 12px",
                    border: masterPasswordError()
                      ? "1px solid #d13438"
                      : "1px solid #ccc",
                    "border-radius": "3px",
                    "font-size": "14px",
                    "box-sizing": "border-box",
                    outline: "none",
                  }}
                  autofocus={true}
                />
                <Show when={masterPasswordError()}>
                  <div
                    style={{
                      color: "#d13438",
                      "font-size": "12px",
                      "margin-top": "5px",
                    }}
                  >
                    {masterPasswordError()}
                  </div>
                </Show>
              </div>

              <div
                style={{
                  display: "flex",
                  gap: "10px",
                  "justify-content": "flex-end",
                }}
              >
                <button
                  onClick={() => {
                    setShowMasterPasswordDialog(false);
                    setMasterPassword("");
                    setMasterPasswordError("");
                    setSelectedPlugin(null);
                  }}
                  style={{
                    padding: "8px 20px",
                    background: "white",
                    color: "#666",
                    border: "1px solid #ccc",
                    "border-radius": "3px",
                    cursor: "pointer",
                    "font-size": "13px",
                  }}
                >
                  取消
                </button>
                <button
                  onClick={verifyMasterPassword}
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
                  {isFirstTimeSetup() ? "设置主密码" : "验证"}
                </button>
              </div>
            </div>
          </div>
        </div>
      </Show>

      {/* 删除确认对话框 */}
      <Show when={showDeleteConfirm()}>
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
              width: "400px",
              "box-shadow": "0 4px 20px rgba(0,0,0,0.3)",
            }}
          >
            <div
              style={{
                padding: "20px",
                "border-bottom": "1px solid #dee2e6",
              }}
            >
              <h2
                style={{
                  margin: 0,
                  "font-size": "18px",
                  "font-weight": "600",
                  color: "#333",
                }}
              >
                确认删除
              </h2>
            </div>

            <div style={{ padding: "20px" }}>
              <p
                style={{
                  margin: "0 0 20px 0",
                  "font-size": "14px",
                  color: "#666",
                  "line-height": "1.5",
                }}
              >
                确定要删除这条密码记录吗?此操作不可恢复!
              </p>

              <div
                style={{
                  display: "flex",
                  gap: "10px",
                  "justify-content": "flex-end",
                }}
              >
                <button
                  onClick={cancelDeletePassword}
                  style={{
                    padding: "8px 20px",
                    background: "white",
                    color: "#666",
                    border: "1px solid #ccc",
                    "border-radius": "3px",
                    cursor: "pointer",
                    "font-size": "13px",
                  }}
                >
                  取消
                </button>
                <button
                  onClick={confirmDeletePassword}
                  style={{
                    padding: "8px 20px",
                    background: "#d13438",
                    color: "white",
                    border: "none",
                    "border-radius": "3px",
                    cursor: "pointer",
                    "font-size": "13px",
                  }}
                >
                  确认删除
                </button>
              </div>
            </div>
          </div>
        </div>
      </Show>

      {/* Toast 提示 */}
      <Show when={showToast()}>
        <div
          style={{
            position: "fixed",
            bottom: "20px",
            left: "50%",
            transform: "translateX(-50%)",
            background: "#333",
            color: "white",
            padding: "12px 24px",
            "border-radius": "4px",
            "box-shadow": "0 2px 8px rgba(0,0,0,0.2)",
            "font-size": "14px",
            "z-index": 2000,
            animation: "fadeIn 0.3s, fadeOut 0.3s 1.7s",
          }}
        >
          {toastMessage()}
        </div>
      </Show>
    </div>
  );
}

export default App;
