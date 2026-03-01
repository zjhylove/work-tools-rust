import { For, Show, createSignal, onMount, createMemo } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import "./AuthPlugin.css";

interface AuthEntry {
  id: string;
  name: string;
  issuer: string;
  secret: string;
  algorithm: string;
  digits: number;
  period: number;
  created_at: string;
  updated_at?: string;
}

interface TotpInfo {
  code: string;
  remaining_seconds: number;
}

export default function AuthPlugin() {
  const [entries, setEntries] = createSignal<AuthEntry[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [viewMode, setViewMode] = createSignal<"list" | "add" | "edit">("list");
  const [selectedEntry, setSelectedEntry] = createSignal<AuthEntry | null>(
    null,
  );
  const [totpMap, setTotpMap] = createSignal<Record<string, TotpInfo>>({});

  // 表单数据
  const [formData, setFormData] = createSignal<Partial<AuthEntry>>({
    name: "",
    issuer: "",
    secret: "",
    algorithm: "SHA1",
    digits: 6,
    period: 30,
  });

  // 字段级错误信息
  const [fieldErrors, setFieldErrors] = createSignal<Record<string, string>>(
    {},
  );

  // 错误信息
  const [error, setError] = createSignal("");
  const [showDeleteConfirm, setShowDeleteConfirm] = createSignal(false);

  // 验证规则定义
  const validationRules = {
    name: { required: true, minLength: 1, label: "账户名称" },
    issuer: { required: true, minLength: 1, label: "发行方" },
    secret: { required: true, minLength: 10, label: "密钥" },
  };

  // 验证单个字段
  const validateField = (key: string, value: string): string | null => {
    const rule = validationRules[key as keyof typeof validationRules];
    if (!rule) return null;

    // 去除首尾空格后再验证
    const trimmedValue = value.trim();

    if (rule.required && !trimmedValue) {
      return `${rule.label}不能为空`;
    }
    // 使用原始值的长度验证（密钥不应有前后空格）
    if (rule.minLength && trimmedValue.length < rule.minLength) {
      return `${rule.label}至少需要 ${rule.minLength} 个字符`;
    }
    return null;
  };

  // 使用 createMemo 创建响应式的表单有效性检查
  const isFormValid = createMemo(() => {
    const data = formData();
    const errors = fieldErrors();

    // 首先检查是否有字段级错误
    if (Object.keys(errors).length > 0) {
      return false;
    }

    // 然后验证所有字段
    for (const [key, rule] of Object.entries(validationRules)) {
      const value = (data[key as keyof AuthEntry] as string) || "";
      const trimmedValue = value.trim();
      if (rule.required && !trimmedValue) {
        return false;
      }
      if (rule.minLength && trimmedValue.length < rule.minLength) {
        return false;
      }
    }
    return true;
  });

  // 加载认证条目列表
  const loadEntries = async () => {
    try {
      setLoading(true);
      const result = await invoke<AuthEntry[]>("list_auth_entries");
      setEntries(result);
    } catch (err) {
      console.error("加载认证条目失败:", err);
      setError("加载认证条目失败");
    } finally {
      setLoading(false);
    }
  };

  // 生成 TOTP 验证码
  const generateTotp = async (entry: AuthEntry) => {
    try {
      console.log("生成 TOTP:", entry.issuer, entry.secret);
      const code = await invoke<string>("generate_totp_code", {
        secret: entry.secret,
        digits: entry.digits,
        period: entry.period,
      });
      console.log("生成的验证码:", entry.issuer, code);

      // 计算剩余时间
      const now = Math.floor(Date.now() / 1000);
      const remaining = entry.period - (now % entry.period);

      setTotpMap((prev) => ({
        ...prev,
        [entry.id]: { code, remaining_seconds: remaining },
      }));
    } catch (err) {
      console.error("生成验证码失败:", entry.issuer, err);
    }
  };

  // 刷新所有验证码
  const refreshAllCodes = () => {
    entries().forEach((entry) => generateTotp(entry));
  };

  // 自动刷新验证码
  onMount(() => {
    loadEntries().then(() => {
      // 加载完条目后,立即生成所有验证码
      setTimeout(() => refreshAllCodes(), 100);
    });

    // 每秒刷新倒计时
    const interval = setInterval(() => {
      const updatedMap: Record<string, TotpInfo> = {};

      entries().forEach((entry) => {
        const current = totpMap()[entry.id];
        if (current) {
          const newRemaining = current.remaining_seconds - 1;
          if (newRemaining <= 0) {
            // 重新生成验证码
            generateTotp(entry);
          } else {
            updatedMap[entry.id] = {
              code: current.code,
              remaining_seconds: newRemaining,
            };
          }
        }
      });

      setTotpMap((prev) => ({ ...prev, ...updatedMap }));
    }, 1000);

    return () => clearInterval(interval);
  });

  // 复制验证码
  const copyCode = async (code: string) => {
    try {
      await navigator.clipboard.writeText(code);
      setError("✓ 验证码已复制");
      setTimeout(() => setError(""), 2000);
    } catch (err) {
      console.error("复制失败:", err);
      setError("复制失败");
    }
  };

  // 保存认证条目
  const saveEntry = async () => {
    try {
      const data = formData();

      // 验证所有必填字段
      const errors: Record<string, string> = {};
      for (const [key, rule] of Object.entries(validationRules)) {
        const value = (data[key as keyof AuthEntry] as string) || "";
        const error = validateField(key, value);
        if (error) {
          errors[key] = error;
        }
      }

      if (Object.keys(errors).length > 0) {
        setFieldErrors(errors);
        setError("请修正表单中的错误");
        return;
      }

      if (viewMode() === "add") {
        await invoke("add_auth_entry", {
          entry: {
            ...data,
            id: "", // 后端会生成
            created_at: new Date().toISOString(),
          },
        });
      } else if (viewMode() === "edit" && selectedEntry()) {
        await invoke("update_auth_entry", {
          entry: {
            ...data,
            id: selectedEntry()!.id,
            created_at: selectedEntry()!.created_at,
          },
        });
      }

      // 重新加载列表
      await loadEntries();
      setViewMode("list");
      setFieldErrors({});
      setError("");
    } catch (err) {
      console.error("保存失败:", err);
      setError("保存认证条目失败");
    }
  };

  // 删除认证条目
  const deleteEntry = async () => {
    if (!selectedEntry()) return;

    try {
      await invoke("delete_auth_entry_plugin", {
        id: selectedEntry()!.id,
      });

      await loadEntries();
      setShowDeleteConfirm(false);
      setSelectedEntry(null);
      setViewMode("list");
    } catch (err) {
      console.error("删除失败:", err);
      setError("删除认证条目失败");
    }
  };

  // 编辑条目
  const editEntry = (entry: AuthEntry) => {
    setSelectedEntry(entry);
    setFormData(entry);
    setFieldErrors({});
    setViewMode("edit");
  };

  // 添加新条目
  const addNew = () => {
    setSelectedEntry(null);
    setFormData({
      name: "",
      issuer: "",
      secret: "",
      algorithm: "SHA1",
      digits: 6,
      period: 30,
    });
    setFieldErrors({});
    setViewMode("add");
    setError("");
  };

  // 生成随机密钥
  const generateSecret = async () => {
    try {
      const secret = await invoke<string>("generate_secret");
      setFormData((prev) => ({ ...prev, secret }));
    } catch (err) {
      console.error("生成密钥失败:", err);
    }
  };

  return (
    <div class="auth-plugin">
      <Show when={viewMode() === "list"}>
        <div class="auth-plugin-header">
          <h2>双因素认证</h2>
          <button class="btn-primary" onClick={addNew}>
            + 添加
          </button>
        </div>
      </Show>

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
        <div class="auth-list">
          <Show
            when={!loading() && entries().length > 0}
            fallback={<div class="empty-state">暂无认证条目</div>}
          >
            <For each={entries()}>
              {(entry) => (
                <div class="auth-item">
                  <div class="auth-item-info">
                    <div class="auth-item-issuer">{entry.issuer}</div>
                    <div class="auth-item-name">{entry.name}</div>
                  </div>

                  <Show when={totpMap()[entry.id]}>
                    {(totp) => (
                      <div class="auth-item-totp">
                        <div class="totp-code">{totp().code}</div>
                        <div class="totp-timer">
                          剩余 {totp().remaining_seconds} 秒
                        </div>
                      </div>
                    )}
                  </Show>

                  <div class="auth-item-actions">
                    <button
                      class="btn-icon"
                      onClick={() => copyCode(totpMap()[entry.id]?.code || "")}
                      title="复制验证码"
                    >
                      📋
                    </button>
                    <button
                      class="btn-icon"
                      onClick={() => editEntry(entry)}
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
      </Show>

      {/* 表单视图 */}
      <Show when={viewMode() === "add" || viewMode() === "edit"}>
        <div class="auth-form-container">
          <div class="auth-form-content">
            {/* 标题栏 */}
            <div class="auth-form-header">
              <h2>{viewMode() === "add" ? "添加认证" : "编辑认证"}</h2>
              <button class="btn-secondary" onClick={() => setViewMode("list")}>
                ✕ 返回列表
              </button>
            </div>

            <div class="form-group">
              <label>发行方 *</label>
              <input
                type="text"
                value={formData().issuer || ""}
                onInput={(e) => {
                  const value = e.currentTarget.value;
                  setFormData((prev) => ({ ...prev, issuer: value }));
                  const error = validateField("issuer", value);
                  setFieldErrors((prev) => {
                    const newErrors = { ...prev };
                    if (error) newErrors.issuer = error;
                    else delete newErrors.issuer;
                    return newErrors;
                  });
                }}
                placeholder="例如: Google"
                classList={{ "input-error": !!fieldErrors().issuer }}
              />
              <Show when={fieldErrors().issuer}>
                <div class="field-error">{fieldErrors().issuer}</div>
              </Show>
            </div>

            <div class="form-group">
              <label>账户名称 *</label>
              <input
                type="text"
                value={formData().name || ""}
                onInput={(e) => {
                  const value = e.currentTarget.value;
                  setFormData((prev) => ({ ...prev, name: value }));
                  const error = validateField("name", value);
                  setFieldErrors((prev) => {
                    const newErrors = { ...prev };
                    if (error) newErrors.name = error;
                    else delete newErrors.name;
                    return newErrors;
                  });
                }}
                placeholder="例如: user@example.com"
                classList={{ "input-error": !!fieldErrors().name }}
              />
              <Show when={fieldErrors().name}>
                <div class="field-error">{fieldErrors().name}</div>
              </Show>
            </div>

            <div class="form-group">
              <label>密钥 *</label>
              <div class="input-with-button">
                <input
                  type="text"
                  value={formData().secret || ""}
                  onInput={(e) => {
                    const value = e.currentTarget.value;
                    setFormData((prev) => ({ ...prev, secret: value }));
                    const error = validateField("secret", value);
                    setFieldErrors((prev) => {
                      const newErrors = { ...prev };
                      if (error) newErrors.secret = error;
                      else delete newErrors.secret;
                      return newErrors;
                    });
                  }}
                  placeholder="输入或生成密钥"
                  classList={{ "input-error": !!fieldErrors().secret }}
                />
                <button class="btn-secondary" onClick={generateSecret}>
                  生成
                </button>
              </div>
              <Show when={fieldErrors().secret}>
                <div class="field-error">{fieldErrors().secret}</div>
              </Show>
            </div>

            <div class="form-row">
              <div class="form-group">
                <label>算法</label>
                <select
                  value={formData().algorithm || "SHA1"}
                  onChange={(e) =>
                    setFormData((prev) => ({
                      ...prev,
                      algorithm: e.currentTarget.value,
                    }))
                  }
                >
                  <option value="SHA1">SHA1</option>
                  <option value="SHA256">SHA256</option>
                  <option value="SHA512">SHA512</option>
                </select>
              </div>

              <div class="form-group">
                <label>位数</label>
                <input
                  type="number"
                  value={formData().digits || 6}
                  onInput={(e) =>
                    setFormData((prev) => ({
                      ...prev,
                      digits: parseInt(e.currentTarget.value) || 6,
                    }))
                  }
                />
              </div>

              <div class="form-group">
                <label>周期(秒)</label>
                <input
                  type="number"
                  value={formData().period || 30}
                  onInput={(e) =>
                    setFormData((prev) => ({
                      ...prev,
                      period: parseInt(e.currentTarget.value) || 30,
                    }))
                  }
                />
              </div>
            </div>

            <div class="form-actions">
              <button
                class="btn-primary"
                onClick={saveEntry}
                disabled={!isFormValid()}
                classList={{ disabled: !isFormValid() }}
              >
                {viewMode() === "add" ? "添加" : "保存"}
              </button>
              <button class="btn-secondary" onClick={() => setViewMode("list")}>
                取消
              </button>
            </div>
          </div>
        </div>
      </Show>

      {/* 删除确认对话框 */}
      <Show when={showDeleteConfirm()}>
        <div class="modal-overlay">
          <div class="modal">
            <h3>确认删除</h3>
            <p>确定要删除这个认证条目吗?</p>
            <div class="modal-actions">
              <button class="btn-danger" onClick={deleteEntry}>
                删除
              </button>
              <button
                class="btn-secondary"
                onClick={() => setShowDeleteConfirm(false)}
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
