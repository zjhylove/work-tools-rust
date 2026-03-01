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
  const [loading, setLoading] = createSignal(true);
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

  // 主密码验证状态
  const [isAuthenticated, setIsAuthenticated] = createSignal(false);
  const [showMasterPasswordPrompt, setShowMasterPasswordPrompt] =
    createSignal(false);
  const [masterPassword, setMasterPassword] = createSignal("");
  const [masterPasswordError, setMasterPasswordError] = createSignal("");
  const [isFirstTimeSetup, setIsFirstTimeSetup] = createSignal(false);

  // 检查主密码状态
  const checkMasterPasswordStatus = async () => {
    try {
      const hasPassword = await invoke<boolean>("has_master_password");
      setIsFirstTimeSetup(!hasPassword);
      return hasPassword;
    } catch (err) {
      console.error("检查主密码状态失败:", err);
      return false;
    }
  };

  // 验证主密码
  const verifyMasterPassword = async () => {
    try {
      const password = masterPassword();
      if (!password || password.length < 6) {
        setMasterPasswordError("主密码至少需要 6 个字符");
        return false;
      }

      console.log("开始验证主密码...");
      const result = await invoke<boolean>("init_or_verify_master_password", {
        password,
      });

      if (result) {
        console.log("主密码验证成功,设置认证状态并加载密码列表");
        setIsAuthenticated(true);
        setShowMasterPasswordPrompt(false);
        setMasterPassword("");
        setMasterPasswordError("");
        setError(""); // 清除之前的错误信息

        // 验证成功后自动加载密码列表
        const loadSuccess = await loadPasswords();
        console.log(
          "密码列表加载",
          loadSuccess ? "成功" : "失败",
          "当前条目数:",
          entries().length,
        );

        return true;
      } else {
        setMasterPasswordError("密码错误");
        return false;
      }
    } catch (err) {
      console.error("主密码验证失败:", err);
      setMasterPasswordError("验证失败: " + err);
      return false;
    }
  };

  // 加载密码列表
  const loadPasswords = async () => {
    try {
      console.log("开始加载密码列表...");
      setLoading(true);
      const result = await invoke<PasswordEntry[]>("get_password_entries");
      console.log("密码列表加载成功,条目数:", result.length);
      setEntries(result);
      setError("");
      // 如果密码列表加载成功,说明后端已验证,设置前端认证状态
      if (!isAuthenticated()) {
        console.log("密码列表加载成功,设置前端认证状态为已验证");
        setIsAuthenticated(true);
      }
      return true; // 返回成功标志
    } catch (err) {
      console.error("加载密码失败:", err);
      const errorMsg = String(err).includes("主密码验证失败")
        ? "请输入主密码以访问密码管理器"
        : "加载密码列表失败";
      setError(errorMsg);
      // 如果是主密码验证失败,显示验证对话框
      if (String(err).includes("主密码验证失败")) {
        console.log("主密码验证失败,显示验证对话框");
        setShowMasterPasswordPrompt(true);
      }
      return false; // 返回失败标志
    } finally {
      setLoading(false);
    }
  };

  // 初始化
  onMount(async () => {
    console.log("PasswordManager 组件挂载,开始初始化...");
    // 检查是否已设置主密码
    const hasPassword = await checkMasterPasswordStatus();
    console.log("主密码状态检查结果:", hasPassword);
    if (!hasPassword) {
      console.log("首次使用,显示设置主密码对话框");
      setShowMasterPasswordPrompt(true);
      return;
    }

    console.log("主密码已设置,尝试加载密码列表...");
    // 尝试加载密码列表,如果失败则提示验证
    await loadPasswords();
    console.log("PasswordManager 初始化完成");
    console.log("当前认证状态:", isAuthenticated());
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
    // 检查是否已验证主密码
    if (!isAuthenticated()) {
      setShowMasterPasswordPrompt(true);
      return;
    }

    setSelectedEntry(null);
    setIsEditMode(false);
    setFormData({});
    setFormErrors({});
    setViewMode("form");
  };

  // 选择条目编辑
  const handleSelectEntry = async (entry: PasswordEntry) => {
    // 检查是否已验证主密码
    if (!isAuthenticated()) {
      setShowMasterPasswordPrompt(true);
      return;
    }

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
    // 检查是否已验证主密码
    if (!isAuthenticated()) {
      setShowMasterPasswordPrompt(true);
      return;
    }

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

      {/* 主密码验证对话框 */}
      <Show when={showMasterPasswordPrompt()}>
        <div class="modal-overlay">
          <div class="modal">
            <h3>{isFirstTimeSetup() ? "设置主密码" : "输入主密码"}</h3>
            <p>
              {isFirstTimeSetup()
                ? "首次使用需要设置主密码,主密码将用于加密所有密码数据。"
                : "请输入主密码以访问密码管理器。"}
            </p>
            <div class="form-group">
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
                classList={{ "input-error": !!masterPasswordError() }}
              />
              <Show when={masterPasswordError()}>
                <div class="field-error">{masterPasswordError()}</div>
              </Show>
            </div>
            <div class="modal-actions">
              <button
                class="btn-primary"
                onClick={async () => {
                  await verifyMasterPassword();
                }}
              >
                {isFirstTimeSetup() ? "设置主密码" : "验证"}
              </button>
              <button
                class="btn-secondary"
                onClick={() => {
                  setShowMasterPasswordPrompt(false);
                  setMasterPassword("");
                  setMasterPasswordError("");
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
