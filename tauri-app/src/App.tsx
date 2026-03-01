import { For, Show, createSignal, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { openUrl as openUrlTauri } from "@tauri-apps/plugin-opener";
import { save, open } from "@tauri-apps/plugin-dialog";
import { writeTextFile, readTextFile } from "@tauri-apps/plugin-fs";
import AuthPlugin from "./components/AuthPlugin";
import PasswordManager from "./components/PasswordManager";
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

// 常量定义
const JSON_FILE_FILTER = { name: "JSON", extensions: ["json"] };
const DEFAULT_EXPORT_PATH = "passwords.json";

// 高阶函数:认证检查包装器
const withAuthCheck = (
  authCheck: () => boolean,
  authPrompt: () => Promise<void>,
) => {
  return <T extends any[], R>(fn: (...args: T) => Promise<R>) => {
    return async (...args: T): Promise<R> => {
      if (!authCheck()) {
        await authPrompt();
        throw new Error("未认证");
      }
      return fn(...args);
    };
  };
};

// 高阶函数:错误处理包装器
const withErrorToast = (toastFn: (message: string) => void) => {
  return <T extends any[]>(fn: (...args: T) => Promise<void>) => {
    return async (...args: T): Promise<void> => {
      try {
        await fn(...args);
      } catch (error) {
        console.error("操作失败:", error);
        toastFn("✗ 操作失败");
      }
    };
  };
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
  // 以下状态已废弃 - 现在由各个插件组件自己管理
  const [pluginView, setPluginView] = createSignal<ViewSchema | null>(null);
  const [formData, setFormData] = createSignal<Record<string, string>>({});
  const [passwordEntries, setPasswordEntries] = createSignal<PasswordEntry[]>(
    [],
  );
  const [selectedEntry, setSelectedEntry] = createSignal<PasswordEntry | null>(
    null,
  );
  const [searchQuery, setSearchQuery] = createSignal("");
  const [isEditMode, setIsEditMode] = createSignal(false);
  const [visiblePasswords, setVisiblePasswords] = createSignal<
    Record<string, boolean>
  >({});
  const [viewMode, setViewMode] = createSignal<"list" | "form">("list");

  // 对话框状态
  const [showLogs, setShowLogs] = createSignal(false);
  const [showPluginMarket, setShowPluginMarket] = createSignal(false);

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

    // 插件组件自己负责数据加载和验证
    // App.tsx 只负责路由和主密码验证
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

      const data = formData();
      const entry: PasswordEntry = {
        id: isEditMode() && selectedEntry() ? selectedEntry()!.id : "", // 新增时使用空字符串,让后端生成 ID
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

  // 导出密码 (优化版本 - 使用高阶函数组合)
  const handleExportPasswords = withErrorToast(showToastMessage)(
    withAuthCheck(
      isAuthenticated,
      showMasterPasswordPrompt,
    )(async () => {
      const jsonData = await safeInvoke<string>("export_passwords");
      const filePath = await save({
        filters: [JSON_FILE_FILTER],
        defaultPath: DEFAULT_EXPORT_PATH,
      });

      if (filePath) {
        await writeTextFile(filePath, jsonData);
        showToastMessage("✓ 导出成功!");
      }
    }),
  );

  // 导入密码 (优化版本 - 使用高阶函数组合)
  const handleImportPasswords = withErrorToast(showToastMessage)(
    withAuthCheck(
      isAuthenticated,
      showMasterPasswordPrompt,
    )(async () => {
      const filePath = await open({
        filters: [JSON_FILE_FILTER],
        multiple: false,
      });

      if (filePath) {
        const jsonData = await readTextFile(filePath);
        await safeInvoke("import_passwords", { jsonData });

        // 刷新密码列表
        const entries = await safeInvoke<PasswordEntry[]>(
          "get_password_entries",
        );
        setPasswordEntries(entries);

        showToastMessage("✓ 导入成功!");
      }
    }),
  );

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
          width: "260px",
          display: "flex",
          "flex-direction": "column",
          "flex-shrink": 0,
        }}
      >
        {/* 插件列表 */}
        <Show when={!loading()}>
          <div
            style={{
              flex: 1,
              overflow: "auto",
              padding: "8px",
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
                    padding: "12px 14px",
                    cursor: "pointer",
                    "user-select": "none",
                    "border-radius": "8px",
                    margin: "0 0 4px 0",
                    background:
                      selectedPlugin() === plugin.id
                        ? "var(--accent-light)"
                        : "transparent",
                    border:
                      selectedPlugin() === plugin.id
                        ? "1px solid var(--accent)"
                        : "1px solid transparent",
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
                      gap: "12px",
                    }}
                  >
                    <span
                      style={{
                        "font-size": "28px",
                        width: "40px",
                        height: "40px",
                        display: "flex",
                        "align-items": "center",
                        "justify-content": "center",
                        background:
                          selectedPlugin() === plugin.id
                            ? "var(--accent)"
                            : "var(--bg-tertiary)",
                        "border-radius": "8px",
                      }}
                    >
                      {plugin.icon}
                    </span>
                    <div style={{ flex: 1 }}>
                      <div
                        style={{
                          "font-size": "14px",
                          "font-weight": "600",
                          "margin-bottom": "3px",
                        }}
                      >
                        {plugin.name}
                      </div>
                      <div
                        style={{
                          "font-size": "12px",
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
            padding: "12px 16px",
            "border-top": "1px solid var(--border-color)",
            display: "flex",
            "justify-content": "center",
            gap: "12px",
          }}
        >
          <button
            onClick={() => setShowLogs(true)}
            title="查看系统日志"
            style={{
              width: "44px",
              height: "44px",
              background: "var(--bg-tertiary)",
              border: "1px solid var(--border-color)",
              color: "var(--text-primary)",
              cursor: "pointer",
              "border-radius": "10px",
              "font-size": "20px",
              transition: "all 0.2s",
              display: "flex",
              "align-items": "center",
              "justify-content": "center",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = "var(--hover-bg)";
              e.currentTarget.style.transform = "scale(1.05)";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "var(--bg-tertiary)";
              e.currentTarget.style.transform = "scale(1)";
            }}
          >
            📋
          </button>
          <button
            onClick={() => setShowPluginMarket(true)}
            title="打开插件市场"
            style={{
              width: "44px",
              height: "44px",
              background: "var(--bg-tertiary)",
              border: "1px solid var(--border-color)",
              color: "var(--text-primary)",
              cursor: "pointer",
              "border-radius": "10px",
              "font-size": "20px",
              transition: "all 0.2s",
              display: "flex",
              "align-items": "center",
              "justify-content": "center",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = "var(--hover-bg)";
              e.currentTarget.style.transform = "scale(1.05)";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "var(--bg-tertiary)";
              e.currentTarget.style.transform = "scale(1)";
            }}
          >
            🧩
          </button>
        </div>
      </div>

      {/* 右侧内容区 */}
      <div
        style={{
          flex: 1,
          background: "var(--bg-tertiary)",
          overflow: "auto",
          display: "flex",
          "flex-direction": "column",
        }}
      >
        <Show when={selectedPlugin() === "password-manager"}>
          <PasswordManager />
        </Show>

        {/* Auth Plugin 界面 */}
        <Show when={selectedPlugin() === "auth"}>
          <AuthPlugin />
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
          class="toast-message"
          classList={{ success: toastMessage().startsWith("✓") }}
        >
          {toastMessage()}
        </div>
      </Show>
    </div>
  );
}

export default App;
