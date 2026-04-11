import { useState, useEffect, useMemo, useCallback } from "react";
import "./App.css";

// ── Types ──────────────────────────────────────────────

interface RouteRule {
  id: string;
  name: string;
  description: string;
  code_length: number;
  code_prefix: string;
  route_script: string;
  tables: string[];
}

interface RouteResult {
  database: string;
  tables: string[];
  code: string;
  rule_name: string;
  parse_time: string;
}

interface RouteData {
  rules: RouteRule[];
}

interface Toast {
  id: string;
  message: string;
  type: "success" | "error" | "info" | "warning";
}

interface RuleFormData {
  name: string;
  description: string;
  code_length: string;
  code_prefix: string;
  tables: string;
  route_script: string;
}

// ── Constants ──────────────────────────────────────────

const TEMPLATES: Record<string, Omit<RouteRule, "id">> = {
  hash_mod4: {
    name: "Hash 取模 (4库)",
    description: "根据编码 Hash 值对 4 取模路由到不同库",
    code_length: 0,
    code_prefix: "",
    route_script:
      "const hash = code.split('').reduce((a, c) => ((a << 5) - a + c.charCodeAt(0)) | 0, 0);\nconst idx = Math.abs(hash) % 4;\nconst database = `db_${String(idx).padStart(2, '0')}`;",
    tables: ["users", "orders", "products"],
  },
  prefix_route: {
    name: "前缀匹配路由",
    description: "根据编码前缀前两位匹配数据库",
    code_length: 10,
    code_prefix: "",
    route_script:
      "const prefix = code.substring(0, 2).toUpperCase();\nconst database = `db_${prefix}`;",
    tables: ["orders", "items"],
  },
  suffix_mod8: {
    name: "尾号取模 (8库)",
    description: "根据编码最后一位对 8 取模路由",
    code_length: 0,
    code_prefix: "",
    route_script:
      "const last = code.charCodeAt(code.length - 1);\nconst idx = last % 8;\nconst database = `shard_${String(idx).padStart(2, '0')}`;",
    tables: ["records", "logs", "events"],
  },
};

const EMPTY_FORM: RuleFormData = {
  name: "",
  description: "",
  code_length: "10",
  code_prefix: "",
  tables: "",
  route_script: "",
};

// ── Helpers ────────────────────────────────────────────

const devLog = (...args: unknown[]) => {
  if (import.meta.env.DEV) console.log("[db-router]", ...args);
};

const generateId = () => crypto.randomUUID();

const parseTables = (input: string): string[] =>
  input
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);

// ── Component ──────────────────────────────────────────

function App() {
  // ── State ────────────────────────────────────────────
  const [rules, setRules] = useState<RouteRule[]>([]);
  const [inputCode, setInputCode] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [result, setResult] = useState<RouteResult | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [toasts, setToasts] = useState<Toast[]>([]);

  // Modal state
  const [showModal, setShowModal] = useState(false);
  const [editingRule, setEditingRule] = useState<RouteRule | null>(null);
  const [formData, setFormData] = useState<RuleFormData>(EMPTY_FORM);
  const [formErrors, setFormErrors] = useState<Record<string, string>>({});

  // Delete confirmation
  const [deleteTarget, setDeleteTarget] = useState<RouteRule | null>(null);

  // ── Toast helpers ────────────────────────────────────
  const addToast = useCallback(
    (message: string, type: Toast["type"] = "info") => {
      const id = generateId();
      setToasts((prev) => [...prev, { id, message, type }]);
      setTimeout(() => {
        setToasts((prev) => prev.filter((t) => t.id !== id));
      }, 3000);
    },
    [],
  );

  // ── Data persistence via pluginAPI ───────────────────
  const loadRules = useCallback(async () => {
    setIsLoading(true);
    try {
      devLog("Loading rules...");
      const data = (await window.pluginAPI?.call(
        "db-router",
        "get_data",
        {},
      )) as RouteData | null;
      setRules(data?.rules ?? []);
      devLog("Rules loaded:", data?.rules?.length ?? 0);
    } catch (err) {
      devLog("Failed to load rules:", err);
      addToast("加载规则失败", "error");
    } finally {
      setIsLoading(false);
    }
  }, [addToast]);

  const saveRules = useCallback(
    async (updatedRules: RouteRule[]) => {
      try {
        await window.pluginAPI?.call("db-router", "save_data", {
          rules: updatedRules,
        });
        setRules(updatedRules);
      } catch (err) {
        devLog("Failed to save rules:", err);
        addToast("保存规则失败", "error");
      }
    },
    [addToast],
  );

  // ── Initialize ───────────────────────────────────────
  useEffect(() => {
    loadRules();
  }, [loadRules]);

  // ── Filtering ────────────────────────────────────────
  const matchedRules = useMemo(() => {
    // Filter by search query
    let filtered = rules;
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      filtered = filtered.filter(
        (r) =>
          r.name.toLowerCase().includes(q) ||
          r.description.toLowerCase().includes(q),
      );
    }

    // Filter by input code (code_length + code_prefix matching)
    if (inputCode.trim()) {
      const code = inputCode.trim();
      filtered = filtered.filter((rule) => {
        if (rule.code_length > 0 && code.length !== rule.code_length) {
          return false;
        }
        if (rule.code_prefix && !code.startsWith(rule.code_prefix)) {
          return false;
        }
        return true;
      });
    }

    return filtered;
  }, [rules, searchQuery, inputCode]);

  // ── Parse route ──────────────────────────────────────
  const handleParseRoute = async (rule: RouteRule) => {
    if (!inputCode.trim()) {
      addToast("请先输入编码", "warning");
      return;
    }

    try {
      const res = (await window.pluginAPI?.call(
        "db-router",
        "parse_route",
        { code: inputCode.trim(), rule },
      )) as RouteResult;

      setResult(res);
      addToast(`解析成功: ${res.database}`, "success");
    } catch (err) {
      devLog("Parse failed:", err);
      addToast("解析路由失败", "error");
    }
  };

  // ── Copy result ──────────────────────────────────────
  const handleCopyResult = async () => {
    if (!result) return;
    const text = `Database: ${result.database}\nTables: ${result.tables.join(", ")}\nCode: ${result.code}\nRule: ${result.rule_name}`;
    try {
      await navigator.clipboard.writeText(text);
      addToast("已复制到剪贴板", "success");
    } catch {
      // Fallback
      const ta = document.createElement("textarea");
      ta.value = text;
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      document.body.removeChild(ta);
      addToast("已复制到剪贴板", "success");
    }
  };

  // ── CRUD Modal ───────────────────────────────────────
  const openCreateModal = () => {
    setEditingRule(null);
    setFormData(EMPTY_FORM);
    setFormErrors({});
    setShowModal(true);
  };

  const openEditModal = (rule: RouteRule) => {
    setEditingRule(rule);
    setFormData({
      name: rule.name,
      description: rule.description,
      code_length: String(rule.code_length),
      code_prefix: rule.code_prefix,
      tables: rule.tables.join(", "),
      route_script: rule.route_script,
    });
    setFormErrors({});
    setShowModal(true);
  };

  const closeModal = () => {
    setShowModal(false);
    setEditingRule(null);
    setFormData(EMPTY_FORM);
    setFormErrors({});
  };

  const validateForm = (): boolean => {
    const errors: Record<string, string> = {};

    if (!formData.name.trim()) errors.name = "规则名称不能为空";
    if (!formData.route_script.trim()) errors.route_script = "路由脚本不能为空";

    const len = parseInt(formData.code_length, 10);
    if (isNaN(len) || len < 0) errors.code_length = "编码长度必须为非负整数";

    setFormErrors(errors);
    return Object.keys(errors).length === 0;
  };

  const handleSaveRule = async () => {
    if (!validateForm()) return;

    const tables = parseTables(formData.tables);
    const newRule: RouteRule = {
      id: editingRule ? editingRule.id : generateId(),
      name: formData.name.trim(),
      description: formData.description.trim(),
      code_length: parseInt(formData.code_length, 10),
      code_prefix: formData.code_prefix.trim(),
      route_script: formData.route_script.trim(),
      tables,
    };

    let updatedRules: RouteRule[];
    if (editingRule) {
      updatedRules = rules.map((r) => (r.id === editingRule.id ? newRule : r));
      addToast("规则已更新", "success");
    } else {
      updatedRules = [...rules, newRule];
      addToast("规则已创建", "success");
    }

    await saveRules(updatedRules);
    closeModal();
  };

  const handleLoadTemplate = (key: string) => {
    const tpl = TEMPLATES[key];
    if (!tpl) return;
    setFormData({
      name: tpl.name,
      description: tpl.description,
      code_length: String(tpl.code_length),
      code_prefix: tpl.code_prefix,
      tables: tpl.tables.join(", "),
      route_script: tpl.route_script,
    });
    setFormErrors({});
    addToast("已加载模板", "info");
  };

  // ── Delete ───────────────────────────────────────────
  const confirmDelete = async () => {
    if (!deleteTarget) return;
    const updatedRules = rules.filter((r) => r.id !== deleteTarget.id);
    await saveRules(updatedRules);
    addToast(`已删除规则: ${deleteTarget.name}`, "success");
    setDeleteTarget(null);
  };

  // ── Import / Export ──────────────────────────────────
  const handleImport = () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json";
    input.style.position = "absolute";
    input.style.left = "-9999px";

    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) {
        document.body.removeChild(input);
        return;
      }
      try {
        const text = await file.text();
        const parsed = JSON.parse(text);
        let imported: RouteRule[];

        if (Array.isArray(parsed)) {
          imported = parsed.map((r) => ({
            ...r,
            id: r.id || generateId(),
          }));
        } else if (parsed.rules && Array.isArray(parsed.rules)) {
          imported = parsed.rules.map((r: RouteRule) => ({
            ...r,
            id: r.id || generateId(),
          }));
        } else {
          throw new Error("无效的文件格式");
        }

        const merged = [...rules];
        for (const rule of imported) {
          if (!merged.find((r) => r.id === rule.id)) {
            merged.push(rule);
          }
        }

        await saveRules(merged);
        addToast(`已导入 ${imported.length} 条规则`, "success");
      } catch (err) {
        addToast("导入失败: " + (err as Error).message, "error");
      } finally {
        document.body.removeChild(input);
      }
    };

    document.body.appendChild(input);
    input.click();
  };

  const handleExport = () => {
    const data = JSON.stringify({ rules, exportedAt: new Date().toISOString() }, null, 2);
    const blob = new Blob([data], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `db-router-rules-${new Date().toISOString().split("T")[0]}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    addToast("规则已导出", "success");
  };

  // ── Form field update ────────────────────────────────
  const updateField = (key: keyof RuleFormData, value: string) => {
    setFormData((prev) => ({ ...prev, [key]: value }));
    // Clear error on change
    if (formErrors[key]) {
      setFormErrors((prev) => {
        const next = { ...prev };
        delete next[key];
        return next;
      });
    }
  };

  // ── Render ───────────────────────────────────────────
  return (
    <div className="db-router">
      {/* Loading overlay */}
      {isLoading && (
        <div className="loading-overlay">
          <div className="loading-spinner">加载中...</div>
        </div>
      )}

      {/* Toast notifications */}
      <div className="toast-container">
        {toasts.map((t) => (
          <div key={t.id} className={`toast toast-${t.type}`}>
            {t.message}
          </div>
        ))}
      </div>

      {/* Toolbar */}
      <div className="toolbar">
        <div className="toolbar-title">数据库路由</div>
        <div className="toolbar-actions">
          <button className="btn btn-primary" onClick={openCreateModal}>
            新建规则
          </button>
          <button className="btn btn-secondary" onClick={handleImport}>
            导入
          </button>
          <button className="btn btn-secondary" onClick={handleExport}>
            导出
          </button>
        </div>
      </div>

      {/* Main content: two columns */}
      <div className="main-content">
        {/* Left panel: rule list */}
        <div className="left-panel">
          <div className="panel-header">
            <input
              type="text"
              className="search-input"
              placeholder="搜索规则..."
              value={searchQuery}
              onInput={(e) => setSearchQuery((e.target as HTMLInputElement).value)}
            />
          </div>

          <div className="rule-list">
            {matchedRules.length === 0 ? (
              <div className="empty-state">
                <div className="empty-icon">
                  {rules.length === 0 ? "📭" : "🔍"}
                </div>
                <div className="empty-text">
                  {rules.length === 0
                    ? "暂无规则，点击新建规则开始"
                    : "没有匹配的规则"}
                </div>
              </div>
            ) : (
              matchedRules.map((rule) => (
                <div key={rule.id} className="rule-card">
                  <div className="rule-card-info">
                    <div className="rule-card-name">{rule.name}</div>
                    {rule.description && (
                      <div className="rule-card-desc">{rule.description}</div>
                    )}
                    <div className="rule-card-tags">
                      <span className="tag tag-length">
                        长度 {rule.code_length || "不限"}
                      </span>
                      {rule.code_prefix && (
                        <span className="tag tag-prefix">
                          前缀 {rule.code_prefix}
                        </span>
                      )}
                      <span className="tag tag-tables">
                        {rule.tables.length} 张表
                      </span>
                    </div>
                  </div>
                  <div className="rule-card-actions">
                    <button
                      className="btn btn-icon"
                      title="解析路由"
                      onClick={() => handleParseRoute(rule)}
                    >
                      ▶
                    </button>
                    <button
                      className="btn btn-icon"
                      title="编辑规则"
                      onClick={() => openEditModal(rule)}
                    >
                      ✏
                    </button>
                    <button
                      className="btn btn-icon btn-icon-danger"
                      title="删除规则"
                      onClick={() => setDeleteTarget(rule)}
                    >
                      🗑
                    </button>
                  </div>
                </div>
              ))
            )}
          </div>

          <div className="panel-footer">
            共 {rules.length} 条规则
            {(searchQuery || inputCode) && (
              <span> / 匹配 {matchedRules.length} 条</span>
            )}
          </div>
        </div>

        {/* Right panel: parser workspace */}
        <div className="right-panel">
          <div className="workspace-section">
            <label className="section-label">输入编码</label>
            <textarea
              className="code-input"
              placeholder="输入需要路由的编码，将自动匹配左侧规则..."
              value={inputCode}
              onInput={(e) => setInputCode((e.target as HTMLTextAreaElement).value)}
              rows={3}
            />
          </div>

          {/* Result card */}
          {result && (
            <div className="result-card">
              <div className="result-header">
                <span className="result-label">解析结果</span>
                <button
                  className="btn btn-sm btn-secondary"
                  onClick={handleCopyResult}
                >
                  复制
                </button>
              </div>
              <div className="result-body">
                <div className="result-database">
                  <span className="result-field-label">Database</span>
                  <span className="result-database-value">
                    {result.database}
                  </span>
                </div>
                <div className="result-tables">
                  <span className="result-field-label">Tables</span>
                  <div className="result-tables-list">
                    {result.tables.map((t, i) => (
                      <span key={i} className="result-table-tag">
                        {t}
                      </span>
                    ))}
                  </div>
                </div>
                <div className="result-meta">
                  <span>规则: {result.rule_name}</span>
                  <span>时间: {result.parse_time}</span>
                </div>
              </div>
            </div>
          )}

          {/* Empty result state */}
          {!result && (
            <div className="result-empty">
              <div className="result-empty-text">
                输入编码后，点击规则卡片的 ▶ 按钮进行解析
              </div>
            </div>
          )}
        </div>
      </div>

      {/* CRUD Modal */}
      {showModal && (
        <div className="modal-overlay" onClick={closeModal}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h3>{editingRule ? "编辑规则" : "新建规则"}</h3>
              <button className="btn btn-icon" onClick={closeModal}>
                ✕
              </button>
            </div>

            <div className="modal-body">
              {/* Template selector */}
              {!editingRule && (
                <div className="form-group">
                  <label className="form-label">加载模板</label>
                  <select
                    className="form-select"
                    value=""
                    onChange={(e) => {
                      if (e.target.value) handleLoadTemplate(e.target.value);
                      e.target.value = "";
                    }}
                  >
                    <option value="">选择模板...</option>
                    {Object.keys(TEMPLATES).map((key) => (
                      <option key={key} value={key}>
                        {TEMPLATES[key].name}
                      </option>
                    ))}
                  </select>
                </div>
              )}

              <div className="form-group">
                <label className="form-label">
                  规则名称 <span className="form-required">*</span>
                </label>
                <input
                  type="text"
                  className={`form-input ${formErrors.name ? "form-input-error" : ""}`}
                  placeholder="例如: Hash 取模路由"
                  value={formData.name}
                  onInput={(e) => updateField("name", (e.target as HTMLInputElement).value)}
                />
                {formErrors.name && (
                  <div className="form-error">{formErrors.name}</div>
                )}
              </div>

              <div className="form-group">
                <label className="form-label">描述</label>
                <input
                  type="text"
                  className="form-input"
                  placeholder="规则用途说明"
                  value={formData.description}
                  onInput={(e) => updateField("description", (e.target as HTMLInputElement).value)}
                />
              </div>

              <div className="form-row">
                <div className="form-group form-group-half">
                  <label className="form-label">编码长度</label>
                  <input
                    type="number"
                    className={`form-input ${formErrors.code_length ? "form-input-error" : ""}`}
                    placeholder="0 = 不限"
                    value={formData.code_length}
                    min={0}
                    onInput={(e) => updateField("code_length", (e.target as HTMLInputElement).value)}
                  />
                  {formErrors.code_length && (
                    <div className="form-error">{formErrors.code_length}</div>
                  )}
                </div>
                <div className="form-group form-group-half">
                  <label className="form-label">编码前缀</label>
                  <input
                    type="text"
                    className="form-input"
                    placeholder="留空 = 不限"
                    value={formData.code_prefix}
                    onInput={(e) => updateField("code_prefix", (e.target as HTMLInputElement).value)}
                  />
                </div>
              </div>

              <div className="form-group">
                <label className="form-label">关联表</label>
                <input
                  type="text"
                  className="form-input"
                  placeholder="用逗号分隔，例如: users, orders, products"
                  value={formData.tables}
                  onInput={(e) => updateField("tables", (e.target as HTMLInputElement).value)}
                />
                <div className="form-hint">多个表名用英文逗号分隔</div>
              </div>

              <div className="form-group">
                <label className="form-label">
                  路由脚本 <span className="form-required">*</span>
                </label>
                <textarea
                  className={`form-textarea ${formErrors.route_script ? "form-input-error" : ""}`}
                  placeholder={`// 变量 code 为输入的编码\n// 返回 database 字符串\nconst database = "db_00";`}
                  value={formData.route_script}
                  onInput={(e) => updateField("route_script", (e.target as HTMLTextAreaElement).value)}
                  rows={6}
                />
                {formErrors.route_script && (
                  <div className="form-error">{formErrors.route_script}</div>
                )}
              </div>
            </div>

            <div className="modal-footer">
              <button className="btn btn-secondary" onClick={closeModal}>
                取消
              </button>
              <button className="btn btn-primary" onClick={handleSaveRule}>
                {editingRule ? "保存修改" : "创建规则"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Delete confirmation */}
      {deleteTarget && (
        <div
          className="modal-overlay"
          onClick={() => setDeleteTarget(null)}
        >
          <div className="modal modal-sm" onClick={(e) => e.stopPropagation()}>
            <h3>确认删除</h3>
            <p>确定要删除规则 "{deleteTarget.name}" 吗？此操作不可撤销。</p>
            <div className="modal-actions">
              <button
                className="btn btn-danger"
                onClick={confirmDelete}
              >
                删除
              </button>
              <button
                className="btn btn-secondary"
                onClick={() => setDeleteTarget(null)}
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
