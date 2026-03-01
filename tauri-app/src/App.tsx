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
  inputType?: string; // 用于密码框等特殊输入类型
  required?: boolean; // 是否必填
  minLength?: number; // 最小长度
  pattern?: string; // 正则表达式模式
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

  // 加载插件列表
  onMount(async () => {
    try {
      const installedPlugins = await invoke<PluginInfo[]>(
        "get_installed_plugins",
      );
      console.log("已安装插件:", installedPlugins);
      setPlugins(installedPlugins);
    } catch (error) {
      console.error("加载插件失败:", error);
      // 开发模式下使用模拟数据
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
      setLoading(false);
    }
  });

  const openPlugin = async (pluginId: string) => {
    console.log("打开插件:", pluginId);
    setSelectedPlugin(pluginId);

    // 如果是密码管理器,加载密码列表
    if (pluginId === "password-manager") {
      try {
        const entries = await invoke<PasswordEntry[]>("get_password_entries");
        setPasswordEntries(entries);
        setSelectedEntry(null);
        setIsEditMode(false);
        setFormData({});
        setFormErrors({});
      } catch (error) {
        console.error("加载密码列表失败:", error);
        setPasswordEntries([]);
      }
    }

    // 模拟 UI Schema (实际应该从插件获取)
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

  const closePlugin = () => {
    setSelectedPlugin(null);
    setPluginView(null);
    setFormData({});
    setFormErrors({});
    setSelectedEntry(null);
    setIsEditMode(false);
  };

  // 验证单个字段
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

  // 验证整个表单
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

  // 检查表单是否有效
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
    console.log(`字段变化: ${key} = ${value}`);

    // 更新表单数据
    setFormData((prev) => ({
      ...prev,
      [key]: value,
    }));

    // 实时验证
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
    console.log("执行操作:", action);

    if (action === "save") {
      // 提交前进行完整验证
      if (!validateForm()) {
        alert("请修正表单中的错误后再提交");
        return;
      }

      // 所有验证通过，保存数据
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
        console.log("密码保存成功:", entry);

        // 重新加载列表
        const entries = await invoke<PasswordEntry[]>("get_password_entries");
        setPasswordEntries(entries);

        alert(isEditMode() ? "密码更新成功!" : "密码保存成功!");

        // 重置表单
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
      console.log("密码删除成功:", id);

      // 重新加载列表
      const entries = await invoke<PasswordEntry[]>("get_password_entries");
      setPasswordEntries(entries);

      // 如果删除的是当前选中的项,清空表单
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

  // 过滤密码条目
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
    <div style="padding: 20px; font-family: Arial, sans-serif; min-height: 100vh; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);">
      <div style="max-width: 900px; margin: 0 auto; background: white; padding: 40px; border-radius: 16px; box-shadow: 0 10px 40px rgba(0,0,0,0.2);">
        {/* 头部 */}
        <Show
          when={!selectedPlugin()}
          fallback={
            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 30px;">
              <button
                onClick={closePlugin}
                style="padding: 10px 20px; background: #6c757d; color: white; border: none; border-radius: 6px; cursor: pointer;"
              >
                ← 返回
              </button>
              <h1 style="color: #333; margin: 0; font-size: 28px;">
                {plugins().find((p) => p.id === selectedPlugin())?.name}
              </h1>
              <div style="width: 80px;"></div>
            </div>
          }
        >
          <div style="text-align: center; margin-bottom: 40px;">
            <h1 style="color: #333; margin: 0 0 10px 0; font-size: 36px;">
              Work Tools Platform
            </h1>
            <p style="color: #666; font-size: 18px; margin: 0;">Rust Edition</p>
            <div style="margin-top: 15px; padding: 10px 20px; background: #d4edda; color: #155724; border-radius: 8px; display: inline-block;">
              ✅ 后端已启动 | 发现 {plugins().length} 个插件
            </div>
          </div>
        </Show>

        {/* 插件列表视图 */}
        <Show when={!selectedPlugin()}>
          <Show when={loading()}>
            <div style="text-align: center; padding: 60px 0;">
              <div style="font-size: 18px; color: #666;">⏳ 加载插件中...</div>
            </div>
          </Show>
          <Show when={!loading()}>
            <div style="margin-top: 30px;">
              <h2 style="color: #555; border-bottom: 3px solid #667eea; padding-bottom: 15px; font-size: 24px;">
                🎯 已安装插件
              </h2>
              <For each={plugins()}>
                {(plugin) => (
                  <div
                    style="
                  padding: 25px;
                  margin: 20px 0;
                  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                  border-radius: 16px;
                  color: white;
                  box-shadow: 0 8px 16px rgba(102, 126, 234, 0.3);
                  transition: all 0.3s ease;
                "
                  >
                    <div style="display: flex; align-items: center; gap: 20px;">
                      <div
                        style="
                      font-size: 48px;
                      width: 80px;
                      height: 80px;
                      display: flex;
                      align-items: center;
                      justify-content: center;
                      background: rgba(255,255,255,0.2);
                      border-radius: 50%;
                    "
                      >
                        {plugin.icon}
                      </div>
                      <div style="flex: 1;">
                        <h3 style="margin: 0 0 8px 0; font-size: 24px;">
                          {plugin.name}
                        </h3>
                        <p style="margin: 8px 0; font-size: 16px; opacity: 0.95; line-height: 1.5;">
                          {plugin.description}
                        </p>
                        <div style="margin-top: 12px; font-size: 13px; opacity: 0.8; font-family: monospace;">
                          版本: {plugin.version} | ID: {plugin.id}
                        </div>
                      </div>
                      <button
                        style="
                        padding: 12px 24px;
                        background: white;
                        color: #667eea;
                        border: none;
                        border-radius: 8px;
                        font-weight: 600;
                        cursor: pointer;
                        transition: all 0.2s;
                      "
                        onClick={() => openPlugin(plugin.id)}
                      >
                        打开插件
                      </button>
                    </div>
                  </div>
                )}
              </For>
            </div>
          </Show>
        </Show>

        {/* 插件详情视图 */}
        <Show when={selectedPlugin() && pluginView()}>
          <div style="margin-top: 20px;">
            {/* 密码管理器专用布局 */}
            <Show when={selectedPlugin() === "password-manager"}>
              <div style="display: flex; gap: 20px; height: 600px;">
                {/* 左侧:密码列表 */}
                <div style="flex: 1; display: flex; flex-direction: column; background: #f8f9fa; border-radius: 12px; overflow: hidden;">
                  {/* 工具栏 */}
                  <div style="padding: 15px; background: white; border-bottom: 1px solid #e0e0e0;">
                    <div style="display: flex; gap: 10px; margin-bottom: 10px;">
                      <button
                        onClick={handleAddNew}
                        style="padding: 8px 16px; background: #28a745; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 14px;"
                      >
                        ➕ 新建
                      </button>
                      <button style="padding: 8px 16px; background: #17a2b8; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 14px;">
                        📥 导入
                      </button>
                      <button style="padding: 8px 16px; background: #6c757d; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 14px;">
                        📤 导出
                      </button>
                    </div>
                    {/* 搜索框 */}
                    <input
                      type="text"
                      placeholder="🔍 搜索密码..."
                      value={searchQuery()}
                      onInput={(e) => setSearchQuery(e.currentTarget.value)}
                      style="width: 100%; padding: 10px; border: 1px solid #e0e0e0; border-radius: 6px; font-size: 14px;"
                    />
                  </div>

                  {/* 密码列表 */}
                  <div style="flex: 1; overflow-y: auto; padding: 10px;">
                    <Show when={filteredEntries().length === 0}>
                      <div style="text-align: center; padding: 60px 20px; color: #999;">
                        <div style="font-size: 48px; margin-bottom: 10px;">
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
                            onClick={() => handleSelectEntry(entry)}
                            style={{
                              padding: "12px 15px",
                              margin: "0 0 8px 0",
                              background:
                                selectedEntry()?.id === entry.id
                                  ? "#667eea"
                                  : "white",
                              color:
                                selectedEntry()?.id === entry.id
                                  ? "white"
                                  : "#333",
                              "border-radius": "8px",
                              cursor: "pointer",
                              border:
                                selectedEntry()?.id === entry.id
                                  ? "2px solid #667eea"
                                  : "1px solid #e0e0e0",
                              transition: "all 0.2s",
                            }}
                            onMouseEnter={(e) => {
                              if (selectedEntry()?.id !== entry.id) {
                                e.currentTarget.style.background = "#f0f0f0";
                              }
                            }}
                            onMouseLeave={(e) => {
                              if (selectedEntry()?.id !== entry.id) {
                                e.currentTarget.style.background = "white";
                              }
                            }}
                          >
                            <div style="font-weight: 600; margin-bottom: 4px;">
                              {entry.service}
                            </div>
                            <div style="font-size: 13px; opacity: 0.8;">
                              {entry.username}
                            </div>
                          </div>
                        )}
                      </For>
                    </Show>
                  </div>

                  {/* 底部统计 */}
                  <div style="padding: 10px 15px; background: white; border-top: 1px solid #e0e0e0; font-size: 13px; color: #666;">
                    共 {passwordEntries().length} 个密码
                    <Show when={searchQuery() !== ""}>
                      <span> / 显示 {filteredEntries().length} 个结果</span>
                    </Show>
                  </div>
                </div>

                {/* 右侧:表单详情 */}
                <div style="flex: 1; background: white; border-radius: 12px; border: 1px solid #e0e0e0; overflow-y: auto;">
                  <div style="padding: 20px;">
                    <h2 style="margin: 0 0 20px 0; color: #333; font-size: 20px;">
                      {isEditMode() ? "编辑密码" : "新建密码"}
                    </h2>

                    <Show when={selectedEntry()}>
                      <div style="margin-bottom: 20px; padding: 15px; background: #fff3cd; border-left: 4px solid #ffc107; border-radius: 4px;">
                        <div style="display: flex; justify-content: space-between; align-items: center;">
                          <span style="color: #856404; font-size: 14px;">
                            正在编辑密码
                          </span>
                          <button
                            onClick={() =>
                              handleDeletePassword(selectedEntry()!.id)
                            }
                            style="padding: 6px 12px; background: #dc3545; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 12px;"
                          >
                            🗑️ 删除
                          </button>
                        </div>
                      </div>
                    </Show>

                    <For each={pluginView()!.fields}>
                      {(field) => (
                        <div style="margin-bottom: 20px;">
                          <Show when={field.type === "input"}>
                            <div>
                              <label style="display: block; margin-bottom: 8px; font-weight: 600; color: #333;">
                                {field.label}
                              </label>
                              <input
                                type={field.inputType || "text"}
                                placeholder={field.placeholder}
                                value={formData()[field.key] || ""}
                                style={{
                                  width: "100%",
                                  padding: "10px 12px",
                                  border: formErrors()[field.key]
                                    ? "2px solid #dc3545"
                                    : "1px solid #e0e0e0",
                                  "border-radius": "6px",
                                  "font-size": "14px",
                                  transition: "border-color 0.2s",
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
                                <div style="margin-top: 5px; color: #dc3545; font-size: 13px;">
                                  {formErrors()[field.key]}
                                </div>
                              </Show>
                            </div>
                          </Show>
                          <Show when={field.type === "button"}>
                            <button
                              onClick={() => handleAction(field.key)}
                              disabled={!isFormValid()}
                              style={{
                                padding: "12px 24px",
                                background: isFormValid()
                                  ? "linear-gradient(135deg, #667eea 0%, #764ba2 100%)"
                                  : "#cccccc",
                                color: "white",
                                border: "none",
                                "border-radius": "6px",
                                "font-weight": "600",
                                cursor: isFormValid()
                                  ? "pointer"
                                  : "not-allowed",
                                "font-size": "15px",
                                transition: "all 0.2s",
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
                      <div style="margin-top: 20px; padding: 15px; background: #f8f9fa; border-radius: 8px;">
                        <div style="font-size: 12px; color: #666; margin-bottom: 5px;">
                          创建时间
                        </div>
                        <div style="font-size: 14px; color: #333;">
                          {new Date(
                            selectedEntry()!.created_at,
                          ).toLocaleString()}
                        </div>
                      </div>
                    </Show>
                  </div>
                </div>
              </div>
            </Show>

            {/* 其他插件使用默认布局 */}
            <Show when={selectedPlugin() !== "password-manager"}>
              <h2 style="color: #555; font-size: 20px; margin-bottom: 20px;">
                插件配置
              </h2>
              <div style="background: #f8f9fa; padding: 30px; border-radius: 12px;">
                <For each={pluginView()!.fields}>
                  {(field) => (
                    <div style="margin-bottom: 20px;">
                      <Show when={field.type === "input"}>
                        <div>
                          <label style="display: block; margin-bottom: 8px; font-weight: 600; color: #333;">
                            {field.label}
                          </label>
                          <input
                            type={field.inputType || "text"}
                            placeholder={field.placeholder}
                            value={formData()[field.key] || ""}
                            style={{
                              width: "100%",
                              padding: "12px",
                              border: formErrors()[field.key]
                                ? "2px solid #dc3545"
                                : "2px solid #e0e0e0",
                              "border-radius": "8px",
                              "font-size": "14px",
                              transition: "border-color 0.2s",
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
                            <div style="margin-top: 5px; color: #dc3545; font-size: 13px;">
                              {formErrors()[field.key]}
                            </div>
                          </Show>
                        </div>
                      </Show>
                      <Show when={field.type === "button"}>
                        <button
                          onClick={() => handleAction(field.key)}
                          disabled={!isFormValid()}
                          style={{
                            padding: "12px 24px",
                            background: isFormValid()
                              ? "linear-gradient(135deg, #667eea 0%, #764ba2 100%)"
                              : "#cccccc",
                            color: "white",
                            border: "none",
                            "border-radius": "8px",
                            "font-weight": "600",
                            cursor: isFormValid() ? "pointer" : "not-allowed",
                            "font-size": "16px",
                            transition: "all 0.2s",
                            opacity: isFormValid() ? 1 : 0.6,
                          }}
                        >
                          {field.label}
                        </button>
                      </Show>
                    </div>
                  )}
                </For>
              </div>
            </Show>
          </div>
        </Show>

        {/* 状态信息 */}
        <Show when={!selectedPlugin()}>
          <div style="margin-top: 40px; padding: 20px; background: #f8f9fa; border-radius: 12px; border-left: 5px solid #28a745;">
            <h3 style="margin: 0 0 15px 0; color: #28a745; font-size: 18px;">
              ✅ 项目状态
            </h3>
            <ul style="margin: 0; padding-left: 20px; color: #555; line-height: 1.8;">
              <li>✅ 共享类型库编译成功</li>
              <li>✅ RPC 协议库编译成功</li>
              <li>✅ Tauri 后端启动成功</li>
              <li>✅ 插件管理器初始化成功</li>
              <li>✅ password-manager 插件编译成功</li>
              <li>✅ Solid.js 前端渲染成功</li>
              <li>✅ UI Schema 动态渲染成功</li>
            </ul>
          </div>
        </Show>

        <div style="margin-top: 30px; text-align: center; color: #999; font-size: 14px;">
          <p>🚀 Work Tools Platform (Rust 版本)</p>
          <p>基于 Tauri 2.x + Solid.js + Rust</p>
        </div>
      </div>
    </div>
  );
}

export default App;
