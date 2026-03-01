import { For, Show, createSignal, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import "./PasswordManager.css";

interface PasswordEntry {
  id: string;
  url: string | null;
  service: string;
  username: string;
  password: string;
  created_at: string;
  updated_at: string;
}

interface UiField {
  type: string;
  label: string;
  key: string;
  placeholder?: string;
  inputType?: string;
  required?: boolean;
  minLength?: number;
  pattern?: string;
}

export default function PasswordManager() {
  const [entries, setEntries] = createSignal<PasswordEntry[]>([]);
  const [viewMode, setViewMode] = createSignal<"list" | "form">("list");
  const [selectedEntry, setSelectedEntry] = createSignal<PasswordEntry | null>(
    null,
  );
  const [visiblePasswords, setVisiblePasswords] = createSignal<
    Record<string, boolean>
  >({});
  const [searchQuery, setSearchQuery] = createSignal("");
  const [isEditMode, setIsEditMode] = createSignal(false);

  // 表单数据
  const [formData, setFormData] = createSignal<Record<string, string>>({});
  const [formErrors, setFormErrors] = createSignal<Record<string, string>>({});

  // 定义密码管理器的表单字段
  const passwordFormFields: UiField[] = [
    {
      type: "input",
      label: "服务名称",
      key: "service",
      placeholder: "例如: Google, GitHub",
      required: true,
      minLength: 1,
    },
    {
      type: "input",
      label: "用户名",
      key: "username",
      placeholder: "用户名或邮箱",
      required: true,
      minLength: 1,
    },
    {
      type: "input",
      label: "密码",
      key: "password",
      placeholder: "输入密码",
      required: true,
      minLength: 1,
      inputType: "password",
    },
    {
      type: "input",
      label: "网站链接",
      key: "url",
      placeholder: "https://example.com (可选)",
      required: false,
    },
    {
      type: "button",
      label: "保存",
      key: "submit",
    },
  ];

  // 错误信息
  const [error, setError] = createSignal("");
  const [showDeleteConfirm, setShowDeleteConfirm] = createSignal(false);

  // 加载密码列表
  const loadPasswords = async () => {
    try {
      console.log("开始加载密码列表...");
      const result = await invoke<PasswordEntry[]>("get_password_entries");
      console.log("密码列表加载成功,条目数:", result.length);
      setEntries(result);
      setError("");
      return true;
    } catch (err) {
      console.error("加载密码失败:", err);
      setError("加载密码列表失败");
      return false;
    }
  };

  // 初始化
  onMount(async () => {
    console.log("PasswordManager 组件挂载,开始初始化...");
    await loadPasswords();
    console.log("PasswordManager 初始化完成");
    console.log("当前条目数:", entries().length);
  });

  // 过滤后的条目
  const filteredEntries = () => {
    const query = searchQuery().toLowerCase().trim();
    if (!query) return entries();

    return entries().filter(
      (entry) =>
        entry.service.toLowerCase().includes(query) ||
        entry.username.toLowerCase().includes(query) ||
        (entry.url && entry.url.toLowerCase().includes(query)),
    );
  };

  // 添加新密码
  const handleAddNew = async () => {
    setSelectedEntry(null);
    setIsEditMode(false);
    setFormData({});
    setFormErrors({});
    setViewMode("form");
  };

  // 选择条目编辑
  const handleSelectEntry = async (entry: PasswordEntry) => {
    setSelectedEntry(entry);
    setIsEditMode(true);
    setFormData({
      service: entry.service,
      username: entry.username,
      password: entry.password,
      url: entry.url || "",
    });
    setFormErrors({});
    setViewMode("form");
  };

  // 删除密码
  const handleDeletePassword = async (id: string) => {
    try {
      await invoke("delete_password_entry", { id });
      await loadPasswords();
      setError("");
    } catch (err) {
      console.error("删除密码失败:", err);
      setError("删除密码失败");
    }
  };

  // 切换密码可见性
  const togglePasswordVisibility = (entryId: string) => {
    setVisiblePasswords((prev) => ({
      ...prev,
      [entryId]: !prev[entryId],
    }));
  };

  // 复制密码
  const copyPassword = async (password: string) => {
    try {
      await navigator.clipboard.writeText(password);
      setError("✓ 密码已复制");
      setTimeout(() => setError(""), 2000);
    } catch (err) {
      console.error("复制失败:", err);
      setError("复制失败");
    }
  };

  // 打开 URL
  const handleOpenUrl = async (url: string) => {
    try {
      await openUrl(url);
    } catch (err) {
      console.error("打开链接失败:", err);
      // 降级处理:使用 window.open
      window.open(url, "_blank");
    }
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

  // 表单字段变化
  const handleFieldChange = (key: string, value: string, field: UiField) => {
    setFormData((prev) => ({ ...prev, [key]: value }));

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

  // 表单是否有效
  const isFormValid = () => {
    // 检查所有字段
    for (const field of passwordFormFields) {
      if (field.type === "input") {
        const value = formData()[field.key] || "";
        const error = validateField(field, value);
        if (error) {
          return false;
        }
      }
    }

    return true;
  };

  // 处理表单提交
  const handleAction = async (actionKey: string) => {
    if (actionKey === "submit") {
      try {
        const data = formData();
        const entry: PasswordEntry = {
          id: isEditMode() && selectedEntry() ? selectedEntry()!.id : "",
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

        await invoke("save_password_entry", { entry });
        await loadPasswords();
        setViewMode("list");
        setSelectedEntry(null);
        setIsEditMode(false);
        setFormData({});
        setFormErrors({});
        setError("");
      } catch (err) {
        console.error("保存密码失败:", err);
        setError("保存密码失败");
      }
    }
  };

  // 导出密码
  const handleExportPasswords = async () => {
    try {
      const result = await invoke<string>("export_passwords");
      const blob = new Blob([result], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `passwords-backup-${new Date().toISOString().split("T")[0]}.json`;
      a.click();
      URL.revokeObjectURL(url);
      setError("✓ 密码已导出");
      setTimeout(() => setError(""), 2000);
    } catch (err) {
      console.error("导出失败:", err);
      setError("导出失败");
    }
  };

  // 导入密码
  const handleImportPasswords = async () => {
    try {
      const input = document.createElement("input");
      input.type = "file";
      input.accept = "application/json";
      input.onchange = async (e) => {
        const file = (e.target as HTMLInputElement).files?.[0];
        if (!file) return;

        const text = await file.text();
        await invoke("import_passwords", { data: text });
        await loadPasswords();
        setError("✓ 密码已导入");
        setTimeout(() => setError(""), 2000);
      };
      input.click();
    } catch (err) {
      console.error("导入失败:", err);
      setError("导入失败");
    }
  };

  return (
    <div class="password-manager">
      {/* 错误提示 */}
      <Show when={error()}>
        <div
          class="error-message"
          classList={{ success: error().startsWith("✓") }}
        >
          {error()}
        </div>
      </Show>

      {/* 列表视图 */}
      <Show when={viewMode() === "list"}>
        <div class="password-list-container">
          {/* 工具栏 */}
          <div class="toolbar">
            <div class="toolbar-actions">
              <button class="btn-primary" onClick={handleAddNew}>
                ➕ 新建
              </button>
              <button class="btn-secondary" onClick={handleImportPasswords}>
                📥 导入
              </button>
              <button class="btn-secondary" onClick={handleExportPasswords}>
                📤 导出
              </button>
            </div>
            <input
              type="text"
              class="search-input"
              placeholder="🔍 搜索密码..."
              value={searchQuery()}
              onInput={(e) => setSearchQuery(e.currentTarget.value)}
            />
          </div>

          {/* 密码列表 */}
          <div class="password-list">
            <Show when={filteredEntries().length === 0}>
              <div class="empty-state">
                <div class="empty-icon">📭</div>
                <div class="empty-text">
                  {searchQuery() ? "没有找到匹配的密码" : "还没有保存的密码"}
                </div>
              </div>
            </Show>
            <Show when={filteredEntries().length > 0}>
              <For each={filteredEntries()}>
                {(entry) => (
                  <div class="password-item">
                    <div class="password-item-info">
                      <div class="password-item-service">{entry.service}</div>
                      <div class="password-item-username">{entry.username}</div>
                      <div class="password-item-password">
                        {visiblePasswords()[entry.id]
                          ? entry.password
                          : "••••••••"}
                      </div>
                    </div>
                    <div class="password-item-actions">
                      <button
                        class="btn-icon"
                        onClick={() => togglePasswordVisibility(entry.id)}
                        title={
                          visiblePasswords()[entry.id] ? "隐藏密码" : "显示密码"
                        }
                      >
                        {visiblePasswords()[entry.id] ? "🙈" : "👁️"}
                      </button>
                      <button
                        class="btn-icon"
                        onClick={() => copyPassword(entry.password)}
                        title="复制密码"
                      >
                        📋
                      </button>
                      <Show when={entry.url}>
                        <button
                          class="btn-icon"
                          onClick={() => handleOpenUrl(entry.url!)}
                          title="打开链接"
                        >
                          🔗
                        </button>
                      </Show>
                      <button
                        class="btn-icon"
                        onClick={() => handleSelectEntry(entry)}
                        title="编辑"
                      >
                        ✏️
                      </button>
                      <button
                        class="btn-icon btn-danger"
                        onClick={() => {
                          setSelectedEntry(entry);
                          setShowDeleteConfirm(true);
                        }}
                        title="删除"
                      >
                        🗑️
                      </button>
                    </div>
                  </div>
                )}
              </For>
            </Show>
          </div>

          {/* 底部统计 */}
          <div class="list-footer">
            共 {entries().length} 个密码
            <Show when={searchQuery() !== ""}>
              <span> / 显示 {filteredEntries().length} 个结果</span>
            </Show>
          </div>
        </div>
      </Show>

      {/* 表单视图 */}
      <Show when={viewMode() === "form"}>
        <div class="password-form-container">
          <div class="form-header">
            <h2>{isEditMode() ? "编辑密码" : "新建密码"}</h2>
            <button
              class="btn-secondary"
              onClick={() => {
                setViewMode("list");
                setSelectedEntry(null);
                setIsEditMode(false);
                setFormData({});
                setFormErrors({});
              }}
            >
              ✕ 返回列表
            </button>
          </div>

          <For each={passwordFormFields}>
            {(field) => (
              <div class="form-field">
                <Show when={field.type === "input"}>
                  <div>
                    <label class="field-label">{field.label}</label>
                    <input
                      type={field.inputType || "text"}
                      placeholder={field.placeholder}
                      value={formData()[field.key] || ""}
                      classList={{
                        "field-input": true,
                        "field-input-error": !!formErrors()[field.key],
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
                      <div class="field-error">{formErrors()[field.key]}</div>
                    </Show>
                  </div>
                </Show>
                <Show when={field.type === "button"}>
                  <button
                    class="btn-submit"
                    disabled={!isFormValid()}
                    onClick={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      handleAction(field.key);
                    }}
                    classList={{ disabled: !isFormValid() }}
                  >
                    {isEditMode() ? "💾 更新密码" : field.label}
                  </button>
                </Show>
              </div>
            )}
          </For>

          <Show when={selectedEntry()}>
            <div class="form-meta">
              <div class="meta-label">创建时间</div>
              <div class="meta-value">
                {new Date(selectedEntry()!.created_at).toLocaleString()}
              </div>
            </div>
          </Show>
        </div>
      </Show>

      {/* 删除确认对话框 */}
      <Show when={showDeleteConfirm() && selectedEntry()}>
        <div class="modal-overlay">
          <div class="modal">
            <h3>确认删除</h3>
            <p>确定要删除 "{selectedEntry()!.service}" 的密码吗?</p>
            <div class="modal-actions">
              <button
                class="btn-danger"
                onClick={async () => {
                  await handleDeletePassword(selectedEntry()!.id);
                  setShowDeleteConfirm(false);
                  setSelectedEntry(null);
                }}
              >
                删除
              </button>
              <button
                class="btn-secondary"
                onClick={() => {
                  setShowDeleteConfirm(false);
                  setSelectedEntry(null);
                }}
              >
                取消
              </button>
            </div>
          </div>
        </div>
      </Show>
    </div>
  );
}
