import { useState, useEffect, useMemo, useCallback, useRef } from "react";
import "./App.css";
import "./types";

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

// 开发环境日志工具
const devError = (...args: unknown[]) => {
  if (import.meta.env.DEV) {
    // eslint-disable-next-line no-console
    console.error(...args);
  }
};

function App() {
  const [entries, setEntries] = useState<AuthEntry[]>([]);
  const entriesRef = useRef<AuthEntry[]>([]); // 用于在 setInterval 中访问最新的 entries
  const isMountedRef = useRef(true); // 用于跟踪组件是否已挂载
  const [loading, setLoading] = useState(true);
  const [viewMode, setViewMode] = useState<"list" | "add" | "edit">("list");
  const [selectedEntry, setSelectedEntry] = useState<AuthEntry | null>(null);
  const [totpMap, setTotpMap] = useState<Record<string, TotpInfo>>({});
  const [formData, setFormData] = useState<Partial<AuthEntry>>({
    name: "",
    issuer: "",
    secret: "",
    algorithm: "SHA1",
    digits: 6,
    period: 30,
  });
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [error, setError] = useState("");
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  // 组件卸载时标记
  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  // 同步 entries 到 ref
  useEffect(() => {
    entriesRef.current = entries;
  }, [entries]);

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

  // 使用 useMemo 创建响应式的表单有效性检查
  const isFormValid = useMemo(() => {
    const errors = fieldErrors;

    // 首先检查是否有字段级错误
    if (Object.keys(errors).length > 0) {
      return false;
    }

    // 然后验证所有字段
    for (const [key, rule] of Object.entries(validationRules)) {
      const value = (formData[key as keyof AuthEntry] as string) || "";
      const trimmedValue = value.trim();
      if (rule.required && !trimmedValue) {
        return false;
      }
      if (rule.minLength && trimmedValue.length < rule.minLength) {
        return false;
      }
    }
    return true;
  }, [formData, fieldErrors]);

  // 加载认证条目列表
  const loadEntries = useCallback(async () => {
    try {
      setLoading(true);
      const result = (await window.pluginAPI?.call(
        "auth",
        "list_entries",
        {},
      )) as unknown;

      // 检查组件是否仍然挂载
      if (!isMountedRef.current) return;

      // 验证返回的数据格式
      if (Array.isArray(result)) {
        console.log("[AuthPlugin] 加载了", result.length, "个认证条目");
        setEntries(result);
      } else {
        console.error("[AuthPlugin] 返回数据格式错误:", typeof result, result);
        setEntries([]);
      }
    } catch (err) {
      if (isMountedRef.current) {
        devError("加载认证条目失败:", err);
        setError("加载认证条目失败");
        setEntries([]);
      }
    } finally {
      if (isMountedRef.current) {
        setLoading(false);
      }
    }
  }, []); // 空依赖数组，不依赖任何外部变量

  // 生成 TOTP 验证码
  const generateTotp = useCallback(
    async (entry: AuthEntry, forceRefresh = false) => {
      // 检查组件是否仍然挂载
      if (!isMountedRef.current) return;

      try {
        // 安全:不要记录 TOTP 秘密或验证码

        // 如果是强制刷新，添加小延迟确保时间步已经更新
        if (forceRefresh) {
          await new Promise((resolve) => setTimeout(resolve, 100));
        }

        const response = await window.pluginAPI?.call("auth", "generate_totp", {
          secret: entry.secret,
          digits: entry.digits,
          period: entry.period,
        });
        const code = (response as { code: string }).code;
        // 安全:不要记录验证码

        // 再次检查组件是否仍然挂载
        if (!isMountedRef.current) return;

        // 计算剩余时间
        const now = Math.floor(Date.now() / 1000);
        const remaining = entry.period - (now % entry.period);

        setTotpMap((prev) => ({
          ...prev,
          [entry.id]: { code, remaining_seconds: remaining },
        }));

        // 如果是强制刷新，显示反馈
        if (forceRefresh && isMountedRef.current) {
          setError("✓ 验证码已刷新");
          setTimeout(() => {
            if (isMountedRef.current) setError("");
          }, 1500);
        }
      } catch (err) {
        if (isMountedRef.current) {
          devError("生成验证码失败:", entry.issuer, err);
          setError("生成验证码失败");
        }
      }
    },
    [],
  ); // 空依赖数组,因为不依赖任何外部变量

  // 自动刷新验证码 - 简化版本,只递归调用 setTimeout
  useEffect(() => {
    let cancelled = false;
    let timeoutId: ReturnType<typeof setTimeout> | null = null;

    // 加载初始数据
    const loadInitialData = async () => {
      if (cancelled) return;
      await loadEntries();

      if (cancelled) return;
      // 加载完条目后,立即生成所有验证码
      setTimeout(() => {
        if (cancelled) return;
        entriesRef.current.forEach((entry) => generateTotp(entry));
      }, 100);

      // 递归的定时器 - 每次 tick 只更新倒计时,不触发异步操作
      const tick = () => {
        if (cancelled) return;

        const currentEntries = entriesRef.current;

        setTotpMap((prev) => {
          const updatedMap: Record<string, TotpInfo> = {};
          let needsRefresh: string[] = [];

          currentEntries.forEach((entry) => {
            const current = prev[entry.id];
            if (current) {
              const newRemaining = current.remaining_seconds - 1;
              if (newRemaining <= 0) {
                // 需要刷新验证码 - 计算正确的剩余时间
                const now = Math.floor(Date.now() / 1000);
                const remaining = entry.period - (now % entry.period);
                needsRefresh.push(entry.id);
                updatedMap[entry.id] = {
                  code: current.code,
                  remaining_seconds: remaining,
                };
              } else {
                updatedMap[entry.id] = {
                  code: current.code,
                  remaining_seconds: newRemaining,
                };
              }
            }
          });

          // 在状态更新完成后,异步刷新需要更新的验证码
          if (needsRefresh.length > 0 && !cancelled) {
            // 使用 queueMicrotask 确保在状态更新后执行
            queueMicrotask(() => {
              if (cancelled) return;
              needsRefresh.forEach((id) => {
                const entry = currentEntries.find((e) => e.id === id);
                if (entry) {
                  generateTotp(entry);
                }
              });
            });
          }

          return { ...prev, ...updatedMap };
        });

        // 继续下一次 tick
        if (!cancelled) {
          timeoutId = setTimeout(tick, 1000);
        }
      };

      // 启动定时器
      timeoutId = setTimeout(tick, 1000);
    };

    loadInitialData();

    return () => {
      cancelled = true;
      if (timeoutId !== null) {
        clearTimeout(timeoutId);
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // 空依赖数组，只在组件挂载时运行一次

  // 复制验证码
  const copyCode = async (code: string) => {
    try {
      await navigator.clipboard.writeText(code);
      setError("✓ 验证码已复制");
      setTimeout(() => setError(""), 2000);
    } catch (err) {
      devError("复制失败:", err);
      setError("复制失败");
    }
  };

  // 保存认证条目
  const saveEntry = async () => {
    try {
      // 验证所有必填字段
      const errors: Record<string, string> = {};
      for (const [key, rule] of Object.entries(validationRules)) {
        const value = (formData[key as keyof AuthEntry] as string) || "";
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

      let savedEntry: AuthEntry | null = null;

      if (viewMode === "add") {
        savedEntry = (await window.pluginAPI?.call("auth", "add_entry", {
          entry: {
            ...formData,
            id: "", // 后端会生成
            created_at: new Date().toISOString(),
          },
        })) as AuthEntry;
      } else if (viewMode === "edit" && selectedEntry) {
        savedEntry = (await window.pluginAPI?.call("auth", "update_entry", {
          entry: {
            ...formData,
            id: selectedEntry.id,
            created_at: selectedEntry.created_at,
          },
        })) as AuthEntry;
      }

      // 重新加载列表
      await loadEntries();

      // 如果是新增条目，立即为其生成验证码
      if (savedEntry) {
        setTimeout(() => generateTotp(savedEntry!), 100);
      }

      setViewMode("list");
      setFieldErrors({});
      setError("");
    } catch (err) {
      devError("保存失败:", err);
      setError("保存认证条目失败");
    }
  };

  // 删除认证条目
  const deleteEntry = async () => {
    if (!selectedEntry) return;

    try {
      await window.pluginAPI?.call("auth", "delete_entry", {
        id: selectedEntry.id,
      });

      await loadEntries();
      setShowDeleteConfirm(false);
      setSelectedEntry(null);
      setViewMode("list");
    } catch (err) {
      devError("删除失败:", err);
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
      const secret = (await window.pluginAPI?.call(
        "auth",
        "generate_secret",
        {},
      )) as string;
      setFormData((prev) => ({ ...prev, secret }));

      // 清除密钥字段的验证错误
      setFieldErrors((prev) => {
        const newErrors = { ...prev };
        delete newErrors.secret;
        return newErrors;
      });
    } catch (err) {
      devError("生成密钥失败:", err);
    }
  };

  return (
    <div className="auth-plugin">
      {viewMode === "list" && (
        <div className="auth-plugin-header">
          <h2>双因素认证</h2>
          <button className="btn-primary" onClick={addNew}>
            + 添加
          </button>
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
        <div className="auth-list">
          {!loading && entries.length > 0 ? (
            entries.map((entry) => (
              <div key={entry.id} className="auth-item">
                <div className="auth-item-info">
                  <div className="auth-item-issuer">{entry.issuer}</div>
                  <div className="auth-item-name">{entry.name}</div>
                </div>

                {totpMap[entry.id] && (
                  <div className="auth-item-totp">
                    <div className="totp-code">{totpMap[entry.id].code}</div>
                    <div className="totp-timer">
                      剩余 {totpMap[entry.id].remaining_seconds} 秒
                    </div>
                  </div>
                )}

                <div className="auth-item-actions">
                  <button
                    className="btn-icon"
                    onClick={() => copyCode(totpMap[entry.id]?.code || "")}
                    title="复制验证码"
                  >
                    📋
                  </button>
                  <button
                    className="btn-icon"
                    onClick={() => generateTotp(entry, true)}
                    title="刷新验证码"
                  >
                    🔄
                  </button>
                  <button
                    className="btn-icon"
                    onClick={() => editEntry(entry)}
                    title="编辑"
                  >
                    ✏️
                  </button>
                  <button
                    className="btn-icon btn-danger"
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
            ))
          ) : (
            <div className="empty-state">暂无认证条目</div>
          )}
        </div>
      )}

      {/* 表单视图 */}
      {(viewMode === "add" || viewMode === "edit") && (
        <div className="auth-form-container">
          <div className="auth-form-content">
            {/* 标题栏 */}
            <div className="auth-form-header">
              <h2>{viewMode === "add" ? "添加认证" : "编辑认证"}</h2>
              <button
                className="btn-secondary"
                onClick={() => setViewMode("list")}
              >
                ✕ 返回列表
              </button>
            </div>

            <div className="form-group">
              <label>发行方 *</label>
              <input
                type="text"
                value={formData.issuer || ""}
                onInput={(e) => {
                  const value = (e.target as HTMLInputElement).value;
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
                className={fieldErrors.issuer ? "input-error" : ""}
              />
              {fieldErrors.issuer && (
                <div className="field-error">{fieldErrors.issuer}</div>
              )}
            </div>

            <div className="form-group">
              <label>账户名称 *</label>
              <input
                type="text"
                value={formData.name || ""}
                onInput={(e) => {
                  const value = (e.target as HTMLInputElement).value;
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
                className={fieldErrors.name ? "input-error" : ""}
              />
              {fieldErrors.name && (
                <div className="field-error">{fieldErrors.name}</div>
              )}
            </div>

            <div className="form-group">
              <label>密钥 *</label>
              <div className="input-with-button">
                <input
                  type="text"
                  value={formData.secret || ""}
                  onInput={(e) => {
                    const value = (e.target as HTMLInputElement).value;
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
                  className={fieldErrors.secret ? "input-error" : ""}
                />
                <button className="btn-secondary" onClick={generateSecret}>
                  生成
                </button>
              </div>
              {fieldErrors.secret && (
                <div className="field-error">{fieldErrors.secret}</div>
              )}
            </div>

            <div className="form-row">
              <div className="form-group">
                <label>算法</label>
                <select
                  value={formData.algorithm || "SHA1"}
                  onChange={(e) => {
                    const value = e.target.value;
                    setFormData((prev) => ({
                      ...prev,
                      algorithm: value,
                    }));
                  }}
                >
                  <option value="SHA1">SHA1</option>
                  <option value="SHA256">SHA256</option>
                  <option value="SHA512">SHA512</option>
                </select>
              </div>

              <div className="form-group">
                <label>位数</label>
                <input
                  type="number"
                  value={formData.digits || 6}
                  onChange={(e) => {
                    const value =
                      parseInt((e.target as HTMLInputElement).value) || 6;
                    setFormData((prev) => ({
                      ...prev,
                      digits: value,
                    }));
                  }}
                />
              </div>

              <div className="form-group">
                <label>周期(秒)</label>
                <input
                  type="number"
                  value={formData.period || 30}
                  onChange={(e) => {
                    const value =
                      parseInt((e.target as HTMLInputElement).value) || 30;
                    setFormData((prev) => ({
                      ...prev,
                      period: value,
                    }));
                  }}
                />
              </div>
            </div>

            <div className="form-actions">
              <button
                className="btn-primary"
                onClick={saveEntry}
                disabled={!isFormValid}
                style={
                  !isFormValid ? { opacity: 0.5, cursor: "not-allowed" } : {}
                }
              >
                {viewMode === "add" ? "添加" : "保存"}
              </button>
              <button
                className="btn-secondary"
                onClick={() => setViewMode("list")}
              >
                取消
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 删除确认对话框 */}
      {showDeleteConfirm && (
        <div className="modal-overlay">
          <div className="modal">
            <h3>确认删除</h3>
            <p>确定要删除这个认证条目吗?</p>
            <div className="modal-actions">
              <button className="btn-danger" onClick={deleteEntry}>
                删除
              </button>
              <button
                className="btn-secondary"
                onClick={() => setShowDeleteConfirm(false)}
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
