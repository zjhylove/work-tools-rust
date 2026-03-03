import { useState, useEffect, useMemo } from "react";
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

// 开发环境日志工具
const devLog = (...args: unknown[]) => {
  if (import.meta.env.DEV) {
    // eslint-disable-next-line no-console
    console.log(...args);
  }
};

const devError = (...args: unknown[]) => {
  if (import.meta.env.DEV) {
    // eslint-disable-next-line no-console
    console.error(...args);
  }
};

function App() {
  const [entries, setEntries] = useState<PasswordEntry[]>([]);
  const [viewMode, setViewMode] = useState<"list" | "form">("list");
  const [selectedEntry, setSelectedEntry] = useState<PasswordEntry | null>(
    null,
  );
  const [visiblePasswords, setVisiblePasswords] = useState<
    Record<string, boolean>
  >({});
  const [searchQuery, setSearchQuery] = useState("");
  const [isEditMode, setIsEditMode] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [formData, setFormData] = useState<Record<string, string>>({});
  const [formErrors, setFormErrors] = useState<Record<string, string>>({});
  const [error, setError] = useState("");
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

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

  // 加载密码列表
  const loadPasswords = async () => {
    setIsLoading(true);
    try {
      devLog("开始加载密码列表...");
      const result = (await window.pluginAPI?.call(
        "password-manager",
        "list_passwords",
        {},
      )) as PasswordEntry[];
      devLog("密码列表加载成功,条目数:", result.length);
      setEntries(result || []);
      setError("");
      return true;
    } catch (err) {
      devError("加载密码失败:", err);
      setError("加载密码列表失败");
      return false;
    } finally {
      setIsLoading(false);
    }
  };

  // 初始化
  useEffect(() => {
    const init = async () => {
      devLog("PasswordManager 组件挂载,开始初始化...");
      await loadPasswords();
      devLog("PasswordManager 初始化完成");
      devLog("当前条目数:", entries.length);
    };
    init();
  }, []);

  // 使用 useMemo 优化过滤性能
  const filteredEntries = useMemo(() => {
    const query = searchQuery.toLowerCase().trim();
    if (!query) return entries;

    return entries.filter(
      (entry) =>
        entry.service.toLowerCase().includes(query) ||
        entry.username.toLowerCase().includes(query) ||
        (entry.url && entry.url.toLowerCase().includes(query)),
    );
  }, [entries, searchQuery]);

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
      await window.pluginAPI?.call("password-manager", "delete_password", {
        id,
      });
      await loadPasswords();
      setError("");
    } catch (err) {
      devError("删除密码失败:", err);
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

  // 复制密码 (带降级方案)
  const copyPassword = async (password: string) => {
    try {
      // 优先使用现代 Clipboard API
      if (navigator.clipboard && navigator.clipboard.writeText) {
        await navigator.clipboard.writeText(password);
      } else {
        // 降级方案: 使用传统方法
        // eslint-disable-next-line deprecation/deprecation
        const textarea = document.createElement("textarea");
        textarea.value = password;
        textarea.style.position = "fixed";
        textarea.style.opacity = "0";
        document.body.appendChild(textarea);
        textarea.select();

        // eslint-disable-next-line deprecation/deprecation
        const successful = document.execCommand("copy");
        document.body.removeChild(textarea);

        if (!successful) {
          throw new Error("execCommand failed");
        }
      }
      setError("✓ 密码已复制");
      setTimeout(() => setError(""), 2000);
    } catch (err) {
      devError("复制失败:", err);
      setError("复制失败,请手动复制");
    }
  };

  // 打开 URL
  const handleOpenUrl = async (url: string) => {
    try {
      // 优先使用 pluginAPI.open_url (通过 Tauri shell 插件)
      if (window.pluginAPI?.open_url) {
        await window.pluginAPI.open_url(url);
      } else {
        // 降级处理:使用 window.open
        window.open(url, "_blank");
      }
    } catch (err) {
      devError("打开链接失败:", err);
      // 最终降级处理
      try {
        window.open(url, "_blank");
      } catch (fallbackErr) {
        devError("降级打开链接也失败:", fallbackErr);
      }
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
    // 使用 URL API 验证 URL
    if (field.key === "url" && value) {
      try {
        new URL(value);
      } catch {
        return "请输入有效的 URL (例如: https://example.com)";
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
        const value = formData[field.key] || "";
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
        const data = formData;
        const isEdit = isEditMode;

        // 安全检查:确保必要的字段存在
        if (!data.service || !data.username || !data.password) {
          setError("请填写所有必填字段");
          return;
        }

        const entry: PasswordEntry = {
          id: isEdit && selectedEntry ? selectedEntry.id : crypto.randomUUID(),
          url: data.url || null,
          service: data.service,
          username: data.username,
          password: data.password,
          created_at:
            isEdit && selectedEntry
              ? selectedEntry.created_at
              : new Date().toISOString(),
          updated_at: new Date().toISOString(),
        };

        // 根据是否是编辑模式调用不同的方法
        if (isEdit && selectedEntry) {
          // 更新现有密码
          await window.pluginAPI?.call("password-manager", "update_password", {
            id: entry.id,
            service: entry.service,
            username: entry.username,
            password: entry.password,
            url: entry.url,
          });
        } else {
          // 添加新密码
          await window.pluginAPI?.call("password-manager", "add_password", {
            service: entry.service,
            username: entry.username,
            password: entry.password,
            url: entry.url,
          });
        }
        await loadPasswords();
        setViewMode("list");
        setSelectedEntry(null);
        setIsEditMode(false);
        setFormData({});
        setFormErrors({});
        setError("");
      } catch (err) {
        devError("保存密码失败:", err);
        setError("保存密码失败");
      }
    }
  };

  // 导出密码 (不使用 confirm,直接导出)
  const handleExportPasswords = async () => {
    setError("⏳ 正在导出密码...");

    try {
      if (!window.pluginAPI) {
        throw new Error("pluginAPI 未初始化");
      }

      const result = (await window.pluginAPI.call(
        "password-manager",
        "export_passwords",
        {},
      )) as { data: string };

      if (!result || !result.data) {
        throw new Error("插件返回数据格式错误: " + JSON.stringify(result));
      }

      const blob = new Blob([result.data], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `passwords-backup-${new Date().toISOString().split("T")[0]}.json`;
      document.body.appendChild(a);
      a.click();
      safeRemoveChild(a);
      URL.revokeObjectURL(url);

      setError("✅ 密码已导出 - 请记得安全存储后删除文件");

      // 5秒后清除提示
      setTimeout(() => {
        setError("");
      }, 5000);
    } catch (err) {
      setError("❌ 导出失败: " + (err as Error).message);

      // 8秒后清除错误提示
      setTimeout(() => {
        setError("");
      }, 8000);
    }
  };

  // 安全移除 DOM 元素的辅助函数
  const safeRemoveChild = (element: HTMLElement) => {
    try {
      if (element && element.parentNode) {
        element.parentNode.removeChild(element);
      }
    } catch (err) {
      // 忽略移除错误，元素可能已经被移除
      devError("移除元素失败:", err);
    }
  };

  // 导入密码 (带确认和预览)
  const handleImportPasswords = async () => {
    try {
      const input = document.createElement("input");
      input.type = "file";
      input.accept = "application/json";
      input.style.position = "absolute";
      input.style.left = "-9999px";
      input.style.visibility = "hidden";

      input.onchange = async (e) => {
        const file = (e.target as HTMLInputElement).files?.[0];
        if (!file) {
          safeRemoveChild(input);
          return;
        }

        try {
          const text = await file.text();

          // 预览导入内容
          let preview;
          try {
            const parsed = JSON.parse(text);

            // 支持两种格式：
            // 1. 旧格式：直接是数组 [{"id": "...", "service": "...", ...}]
            // 2. 新格式：{"entries": [{"id": "...", "service": "...", ...}]}
            if (Array.isArray(parsed)) {
              preview = parsed;
            } else if (parsed.entries && Array.isArray(parsed.entries)) {
              preview = parsed.entries;
            } else {
              throw new Error("无效的格式");
            }
          } catch (err) {
            setError("❌ 导入失败: 文件格式不正确 - " + (err as Error).message);
            setTimeout(() => setError(""), 5000);
            safeRemoveChild(input);
            return;
          }

          const count = preview.length;

          if (count === 0) {
            setError("⚠️ 文件中没有密码条目");
            setTimeout(() => setError(""), 3000);
            safeRemoveChild(input);
            return;
          }

          // 显示确认对话框（如果 confirm 可用）
          let confirmed = true;
          try {
            confirmed = confirm(
              `即将导入 ${count} 个密码条目。\n\n` +
                `⚠️ 注意:\n` +
                `• 现有密码可能会被覆盖\n` +
                `• 请确保备份文件来源可信\n\n` +
                `是否继续导入?`,
            );
          } catch {
            // confirm 被阻止，自动继续
          }

          if (!confirmed) {
            safeRemoveChild(input);
            return;
          }

          await window.pluginAPI?.call("password-manager", "import_passwords", {
            data: text,
          });
          await loadPasswords();
          setError(`✅ 已成功导入 ${count} 个密码`);
          setTimeout(() => setError(""), 5000);
        } finally {
          // 安全清理 DOM 元素
          safeRemoveChild(input);
        }
      };

      document.body.appendChild(input);
      input.click();
    } catch (err) {
      devError("导入失败:", err);
      setError("❌ 导入失败: " + (err as Error).message);
      setTimeout(() => setError(""), 5000);
    }
  };

  return (
    <div className="password-manager">
      {/* 加载状态 */}
      {isLoading && (
        <div className="loading-overlay">
          <div className="loading-spinner">加载中...</div>
        </div>
      )}

      {/* 错误提示 */}
      {error && (
        <div
          className="error-message"
          style={
            String(error).startsWith("✓")
              ? { backgroundColor: "#d4edda", color: "#155724" }
              : {}
          }
        >
          {String(error)}
        </div>
      )}

      {/* 列表视图 */}
      {viewMode === "list" && (
        <div className="password-list-container">
          {/* 工具栏 */}
          <div className="toolbar">
            <div className="toolbar-actions">
              <button
                className="btn-primary"
                onClick={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  handleAddNew();
                }}
              >
                ➕ 新建
              </button>
              <button
                className="btn-secondary"
                onClick={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  handleImportPasswords();
                }}
              >
                📥 导入
              </button>
              <button
                className="btn-secondary"
                onClick={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  handleExportPasswords();
                }}
              >
                📤 导出
              </button>
            </div>
            <input
              type="text"
              className="search-input"
              placeholder="🔍 搜索密码..."
              value={searchQuery}
              onInput={(e) =>
                setSearchQuery((e.target as HTMLInputElement).value)
              }
            />
          </div>

          {/* 密码列表 */}
          <div className="password-list">
            {filteredEntries.length === 0 ? (
              <div className="empty-state">
                <div className="empty-icon">📭</div>
                <div className="empty-text">
                  {searchQuery ? "没有找到匹配的密码" : "还没有保存的密码"}
                </div>
              </div>
            ) : (
              filteredEntries.map((entry) => (
                <div key={entry.id} className="password-item">
                  <div className="password-item-info">
                    <div className="password-item-service">{entry.service}</div>
                    <div className="password-item-username">
                      {entry.username}
                    </div>
                    <div className="password-item-password">
                      {visiblePasswords[entry.id] ? entry.password : "••••••••"}
                    </div>
                  </div>
                  <div className="password-item-actions">
                    <button
                      className="btn-icon"
                      onClick={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        togglePasswordVisibility(entry.id);
                      }}
                      title={
                        visiblePasswords[entry.id] ? "隐藏密码" : "显示密码"
                      }
                      aria-label={
                        visiblePasswords[entry.id] ? "隐藏密码" : "显示密码"
                      }
                      role="button"
                    >
                      {visiblePasswords[entry.id] ? "🙈" : "👁️"}
                    </button>
                    <button
                      className="btn-icon"
                      onClick={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        copyPassword(entry.password);
                      }}
                      title="复制密码"
                      aria-label="复制密码"
                      role="button"
                    >
                      📋
                    </button>
                    {entry.url && (
                      <button
                        className="btn-icon"
                        onClick={(e) => {
                          e.preventDefault();
                          e.stopPropagation();
                          if (entry.url) {
                            handleOpenUrl(entry.url);
                          }
                        }}
                        title="打开链接"
                        aria-label="打开链接"
                        role="button"
                      >
                        🔗
                      </button>
                    )}
                    <button
                      className="btn-icon"
                      onClick={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        handleSelectEntry(entry);
                      }}
                      title="编辑"
                      aria-label="编辑密码"
                      role="button"
                    >
                      ✏️
                    </button>
                    <button
                      className="btn-icon btn-danger"
                      onClick={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        setSelectedEntry(entry);
                        setShowDeleteConfirm(true);
                      }}
                      title="删除"
                      aria-label="删除密码"
                      role="button"
                    >
                      🗑️
                    </button>
                  </div>
                </div>
              ))
            )}
          </div>

          {/* 底部统计 */}
          <div className="list-footer">
            共 {entries.length} 个密码
            {searchQuery !== "" && (
              <span> / 显示 {filteredEntries.length} 个结果</span>
            )}
          </div>
        </div>
      )}

      {/* 表单视图 */}
      {viewMode === "form" && (
        <div className="password-form-container">
          <div className="form-header">
            <h2>{isEditMode ? "编辑密码" : "新建密码"}</h2>
            <button
              className="btn-secondary"
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
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

          {passwordFormFields.map((field) => (
            <div key={field.key} className="form-field">
              {field.type === "input" && (
                <div>
                  <label className="field-label">{field.label}</label>
                  <input
                    type={field.inputType || "text"}
                    placeholder={field.placeholder}
                    value={formData[field.key] || ""}
                    className={`field-input ${formErrors[field.key] ? "field-input-error" : ""}`}
                    onInput={(e) =>
                      handleFieldChange(
                        field.key,
                        (e.target as HTMLInputElement).value,
                        field,
                      )
                    }
                  />
                  {formErrors[field.key] && (
                    <div className="field-error">{formErrors[field.key]}</div>
                  )}
                </div>
              )}
              {field.type === "button" && (
                <button
                  className="btn-submit"
                  disabled={!isFormValid()}
                  onClick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    handleAction(field.key);
                  }}
                  style={
                    !isFormValid()
                      ? { opacity: 0.5, cursor: "not-allowed" }
                      : {}
                  }
                >
                  {isEditMode ? "💾 更新密码" : field.label}
                </button>
              )}
            </div>
          ))}

          {selectedEntry && (
            <div className="form-meta">
              <div className="meta-label">创建时间</div>
              <div className="meta-value">
                {new Date(selectedEntry.created_at).toLocaleString()}
              </div>
            </div>
          )}
        </div>
      )}

      {/* 删除确认对话框 */}
      {showDeleteConfirm && selectedEntry && (
        <div className="modal-overlay">
          <div className="modal">
            <h3>确认删除</h3>
            <p>确定要删除 "{selectedEntry.service}" 的密码吗?</p>
            <div className="modal-actions">
              <button
                className="btn-danger"
                onClick={async (e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  await handleDeletePassword(selectedEntry.id);
                  setShowDeleteConfirm(false);
                  setSelectedEntry(null);
                }}
              >
                删除
              </button>
              <button
                className="btn-secondary"
                onClick={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  setShowDeleteConfirm(false);
                  setSelectedEntry(null);
                }}
              >
                取消
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
