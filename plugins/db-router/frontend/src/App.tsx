import { useState, useEffect, useMemo, useCallback } from "react";
import "./App.css";

// --- Types ---

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
  id: number;
  message: string;
  type: "success" | "error" | "info";
}

// --- SVG Icons ---

const Icons = {
  database: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <ellipse cx="12" cy="5" rx="9" ry="3" />
      <path d="M3 5V19A9 3 0 0 0 21 19V5" />
      <path d="M3 12A9 3 0 0 0 21 12" />
    </svg>
  ),
  search: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="11" cy="11" r="8" />
      <line x1="21" y1="21" x2="16.65" y2="16.65" />
    </svg>
  ),
  plus: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <line x1="12" y1="5" x2="12" y2="19" />
      <line x1="5" y1="12" x2="19" y2="12" />
    </svg>
  ),
  download: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
      <polyline points="7 10 12 15 17 10" />
      <line x1="12" y1="15" x2="12" y2="3" />
    </svg>
  ),
  upload: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
      <polyline points="17 8 12 3 7 8" />
      <line x1="12" y1="3" x2="12" y2="15" />
    </svg>
  ),
  play: (
    <svg viewBox="0 0 24 24" fill="currentColor" stroke="none">
      <polygon points="6,3 20,12 6,21" />
    </svg>
  ),
  edit: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
      <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
    </svg>
  ),
  trash: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <polyline points="3 6 5 6 21 6" />
      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
    </svg>
  ),
  copy: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
    </svg>
  ),
  check: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
      <polyline points="22 4 12 14.01 9 11.01" />
    </svg>
  ),
  x: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <line x1="18" y1="6" x2="6" y2="18" />
      <line x1="6" y1="6" x2="18" y2="18" />
    </svg>
  ),
  lightbulb: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M9 18h6" />
      <path d="M10 22h4" />
      <path d="M15.09 14c.18-.98.65-1.74 1.41-2.5A4.65 4.65 0 0 0 18 8 6 6 0 0 0 6 8c0 1 .23 2.23 1.5 3.5A4.61 4.61 0 0 1 8.91 14" />
    </svg>
  ),
  fileCode: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <polyline points="16 18 22 12 16 6" />
      <polyline points="8 6 2 12 8 18" />
    </svg>
  ),
  inbox: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <polyline points="22 12 16 12 14 15 10 15 8 12 2 12" />
      <path d="M5.45 5.11L2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z" />
    </svg>
  ),
  terminal: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <polyline points="4 17 10 11 4 5" />
      <line x1="12" y1="19" x2="20" y2="19" />
    </svg>
  ),
  settings: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </svg>
  ),
  ruler: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M21.3 15.3a2.4 2.4 0 0 1 0 3.4l-2.6 2.6a2.4 2.4 0 0 1-3.4 0L2.7 8.7a2.41 2.41 0 0 1 0-3.4l2.6-2.6a2.41 2.41 0 0 1 3.4 0Z" />
      <path d="m14.5 12.5 2-2" />
      <path d="m11.5 9.5 2-2" />
      <path d="m8.5 6.5 2-2" />
      <path d="m17.5 15.5 2-2" />
    </svg>
  ),
  table: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
      <line x1="3" y1="9" x2="21" y2="9" />
      <line x1="3" y1="15" x2="21" y2="15" />
      <line x1="9" y1="3" x2="9" y2="21" />
    </svg>
  ),
};

// --- Helpers ---

const call = async (method: string, params: Record<string, unknown> = {}) => {
  return await window.pluginAPI?.call("db-router", method, params);
};

const copyToClipboard = async (text: string) => {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const ta = document.createElement("textarea");
    ta.value = text;
    ta.style.position = "fixed";
    ta.style.left = "-9999px";
    document.body.appendChild(ta);
    ta.select();
    document.execCommand("copy");
    document.body.removeChild(ta);
  }
};

// --- App ---

function App() {
  const [rules, setRules] = useState<RouteRule[]>([]);
  const [code, setCode] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [result, setResult] = useState<RouteResult | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);

  const [showModal, setShowModal] = useState(false);
  const [editingRule, setEditingRule] = useState<RouteRule | null>(null);
  const [formData, setFormData] = useState({
    name: "",
    description: "",
    code_length: "",
    code_prefix: "",
    route_script: "",
    tables: "",
  });
  const [showDeleteConfirm, setShowDeleteConfirm] = useState<string | null>(null);
  const [templates, setTemplates] = useState<RouteRule[]>([]);

  const addToast = useCallback((message: string, type: Toast["type"] = "info") => {
    const id = Date.now();
    setToasts((prev) => [...prev, { message, type, id }]);
    setTimeout(() => setToasts((prev) => prev.filter((t) => t.id !== id)), 3000);
  }, []);

  const loadRules = useCallback(async () => {
    try {
      const data = (await call("list_rules")) as RouteData;
      setRules(data?.rules || []);
    } catch {
      addToast("加载规则失败", "error");
    }
  }, [addToast]);

  const loadTemplates = useCallback(async () => {
    try {
      const data = (await call("get_templates")) as RouteRule[];
      setTemplates(data || []);
    } catch {
      // silent
    }
  }, []);

  useEffect(() => {
    loadRules();
    loadTemplates();
  }, [loadRules, loadTemplates]);

  // Check if a rule matches the current code
  const doesCodeMatch = useCallback(
    (rule: RouteRule) => {
      if (!code.trim()) return false;
      if (rule.code_length > 0 && code.length !== rule.code_length) return false;
      if (rule.code_prefix) {
        const prefixes = rule.code_prefix.split(",").map((s) => s.trim()).filter(Boolean);
        if (prefixes.length > 0 && !prefixes.some((p) => code.startsWith(p))) return false;
      }
      return true;
    },
    [code]
  );

  const filteredRules = useMemo(() => {
    let filtered = rules;

    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      filtered = filtered.filter((r) => r.name.toLowerCase().includes(q));
    }

    return filtered;
  }, [rules, searchQuery]);

  const matchedRuleIds = useMemo(() => {
    if (!code.trim()) return new Set<string>();
    return new Set(rules.filter(doesCodeMatch).map((r) => r.id));
  }, [rules, code, doesCodeMatch]);

  const handleParse = async (ruleId: string) => {
    if (!code.trim()) {
      addToast("请输入编号", "error");
      return;
    }
    try {
      const res = (await call("parse_route", { code, rule_id: ruleId })) as RouteResult;
      setResult(res);
    } catch (err) {
      setResult(null);
      addToast(`解析失败: ${(err as Error).message}`, "error");
    }
  };

  const handleOpenNew = () => {
    setEditingRule(null);
    setFormData({ name: "", description: "", code_length: "", code_prefix: "", route_script: "", tables: "" });
    setShowModal(true);
  };

  const handleOpenEdit = (rule: RouteRule) => {
    setEditingRule(rule);
    setFormData({
      name: rule.name,
      description: rule.description,
      code_length: rule.code_length > 0 ? String(rule.code_length) : "",
      code_prefix: rule.code_prefix,
      route_script: rule.route_script,
      tables: rule.tables.join("\n"),
    });
    setShowModal(true);
  };

  const handleSave = async () => {
    if (!formData.name.trim()) {
      addToast("规则名称不能为空", "error");
      return;
    }
    if (!formData.route_script.trim()) {
      addToast("解析脚本不能为空", "error");
      return;
    }

    const rule: RouteRule = {
      id: editingRule?.id || "",
      name: formData.name.trim(),
      description: formData.description.trim(),
      code_length: formData.code_length ? parseInt(formData.code_length) || 0 : 0,
      code_prefix: formData.code_prefix.trim(),
      route_script: formData.route_script.trim(),
      tables: formData.tables
        .split("\n")
        .map((t) => t.trim())
        .filter(Boolean),
    };

    try {
      await call("save_rule", { rule });
      await loadRules();
      setShowModal(false);
      addToast(editingRule ? "规则已更新" : "规则已创建", "success");
    } catch (err) {
      addToast(`保存失败: ${(err as Error).message}`, "error");
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await call("delete_rule", { id });
      await loadRules();
      setShowDeleteConfirm(null);
      if (result && result.rule_name === rules.find((r) => r.id === id)?.name) {
        setResult(null);
      }
      addToast("规则已删除", "success");
    } catch (err) {
      addToast(`删除失败: ${(err as Error).message}`, "error");
    }
  };

  const handleImport = async () => {
    try {
      const input = document.createElement("input");
      input.type = "file";
      input.accept = ".json";
      input.style.position = "absolute";
      input.style.left = "-9999px";
      input.onchange = async (e) => {
        const file = (e.target as HTMLInputElement).files?.[0];
        if (!file) return;
        try {
          const text = await file.text();
          const parsed = JSON.parse(text);
          const rulesArr = Array.isArray(parsed) ? parsed : parsed.rules || [];
          await call("import_rules", { rules: rulesArr });
          await loadRules();
          addToast(`已导入 ${rulesArr.length} 条规则`, "success");
        } catch {
          addToast("导入失败: 文件格式不正确", "error");
        }
        document.body.removeChild(input);
      };
      document.body.appendChild(input);
      input.click();
    } catch {
      addToast("导入失败", "error");
    }
  };

  const handleExport = async () => {
    try {
      const exportData = (await call("export_rules")) as RouteRule[];
      const json = JSON.stringify(exportData, null, 2);
      const blob = new Blob([json], { type: "application/json" });

      const defaultName = `db-router-rules-${new Date().toISOString().split("T")[0]}.json`;

      if ("showSaveFilePicker" in window) {
        const handle = await (window as unknown as { showSaveFilePicker: (opts: unknown) => Promise<FileSystemFileHandle> }).showSaveFilePicker({
          suggestedName: defaultName,
          types: [{ description: "JSON Files", accept: { "application/json": [".json"] } }],
        });
        const writable = await handle.createWritable();
        await writable.write(blob);
        await writable.close();
        addToast(`规则已导出到 ${handle.name}`, "success");
      } else {
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = defaultName;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        addToast("规则已导出", "success");
      }
    } catch (err) {
      if ((err as Error).name !== "AbortError") {
        addToast("导出失败", "error");
      }
    }
  };

  const handleCopyText = async (text: string, label: string) => {
    await copyToClipboard(text);
    addToast(`${label}已复制`, "success");
  };

  const handleLoadTemplate = (templateName: string) => {
    const template = templates.find((t) => t.name === templateName);
    if (template) {
      setFormData((prev) => ({ ...prev, route_script: template.route_script }));
    }
  };

  const toastIcon = (type: Toast["type"]) => {
    switch (type) {
      case "success":
        return Icons.check;
      case "error":
        return Icons.x;
      default:
        return Icons.lightbulb;
    }
  };

  return (
    <div className="db-router">
      {/* Toasts */}
      {toasts.map((t) => (
        <div key={t.id} className={`toast toast-${t.type}`}>
          {toastIcon(t.type)}
          {t.message}
        </div>
      ))}

      {/* Toolbar */}
      <div className="toolbar">
        <div className="toolbar-title">
          {Icons.database}
          数据库路由
        </div>
        <div className="toolbar-actions">
          <button className="btn btn-primary" onClick={handleOpenNew}>
            {Icons.plus}
            新建规则
          </button>
          <button className="btn btn-secondary" onClick={handleImport}>
            {Icons.download}
            导入
          </button>
          <button className="btn btn-secondary" onClick={handleExport}>
            {Icons.upload}
            导出
          </button>
        </div>
      </div>

      {/* Main Content */}
      <div className="main-content">
        {/* Left Panel: Rules */}
        <div className="left-panel">
          <div className="panel-search">
            <div className="search-wrapper">
              {Icons.search}
              <input
                type="text"
                className="search-input"
                placeholder="搜索规则..."
                value={searchQuery}
                onInput={(e) => setSearchQuery((e.target as HTMLInputElement).value)}
              />
            </div>
          </div>
          <div className="rule-list">
            {filteredRules.length === 0 ? (
              <div className="empty-state">
                {Icons.inbox}
                <div className="empty-text">
                  {code || searchQuery ? "没有匹配的规则" : "还没有路由规则"}
                </div>
                {!code && !searchQuery && (
                  <div className="empty-text-sub">点击「新建规则」添加第一条路由规则</div>
                )}
              </div>
            ) : (
              filteredRules.map((rule) => {
                const isMatched = matchedRuleIds.has(rule.id);
                const isDimmed = code.trim() && !isMatched;
                return (
                  <div
                    key={rule.id}
                    className={`rule-card${isMatched ? " rule-matched" : ""}${isDimmed ? " rule-dimmed" : ""}`}
                  >
                    <div className="rule-info">
                      <div className="rule-name">{rule.name}</div>
                      {rule.description && <div className="rule-desc">{rule.description}</div>}
                      <div className="rule-tags">
                        <span className="tag tag-length">
                          {Icons.ruler}
                          {rule.code_length > 0 ? `${rule.code_length}位` : "任意长度"}
                        </span>
                        <span className="tag tag-prefix">
                          {rule.code_prefix ? rule.code_prefix : "任意前缀"}
                        </span>
                        {rule.tables.length > 0 && (
                          <span className="tag tag-tables">
                            {Icons.table}
                            {rule.tables.length}张表
                          </span>
                        )}
                      </div>
                    </div>
                    <div className="rule-actions">
                      <button
                        className="btn-icon btn-icon-parse"
                        onClick={() => handleParse(rule.id)}
                        title="解析"
                      >
                        {Icons.play}
                      </button>
                      <button
                        className="btn-icon btn-icon-edit"
                        onClick={() => handleOpenEdit(rule)}
                        title="编辑"
                      >
                        {Icons.edit}
                      </button>
                      <button
                        className="btn-icon btn-icon-danger"
                        onClick={() => setShowDeleteConfirm(rule.id)}
                        title="删除"
                      >
                        {Icons.trash}
                      </button>
                    </div>
                  </div>
                );
              })
            )}
          </div>
          <div className="panel-footer">
            共 <strong>{rules.length}</strong> 条规则
            {code.trim() && matchedRuleIds.size > 0 && (
              <>，匹配 <strong>{matchedRuleIds.size}</strong> 条</>
            )}
          </div>
        </div>

        {/* Right Panel: Parser */}
        <div className="right-panel">
          <div className="parser-section">
            <label className="section-label">
              {Icons.terminal}
              输入编号
            </label>
            <input
              type="text"
              className="code-input"
              placeholder="输入编号进行路由解析..."
              value={code}
              onInput={(e) => {
                setCode((e.target as HTMLInputElement).value);
                setResult(null);
              }}
            />
            {code.trim() && matchedRuleIds.size > 0 && (
              <div className="section-hint">
                已匹配 {matchedRuleIds.size} 条规则，点击规则卡片上的播放按钮执行解析
              </div>
            )}
          </div>

          <div className="parser-section result-section">
            <label className="section-label">
              {Icons.terminal}
              解析结果
            </label>
            <div className="result-card">
              {!result ? (
                <div className="result-empty">
                  {Icons.search}
                  <div className="result-empty-text">
                    {code.trim() ? "选择匹配的规则执行解析" : "输入编号并选择规则进行解析"}
                  </div>
                </div>
              ) : (
                <>
                  <div className="result-header">
                    <div className="result-header-left">
                      {Icons.check}
                      解析成功
                    </div>
                    <span className="result-rule">via {result.rule_name}</span>
                  </div>
                  <div className="result-body">
                    <div className="result-field">
                      <div className="result-field-label">DATABASE</div>
                      <div className="result-field-value">
                        {result.database}
                        <button
                          className="btn-copy-inline"
                          onClick={() => handleCopyText(result.database, "数据库名")}
                          title="复制"
                        >
                          {Icons.copy}
                        </button>
                      </div>
                    </div>
                    <div className="result-field">
                      <div className="result-field-label">
                        TABLES
                        {result.tables.length > 0 && (
                          <span className="table-count"> ({result.tables.length})</span>
                        )}
                      </div>
                      <div className="result-tables">
                        {result.tables.map((t, i) => (
                          <div
                            key={i}
                            className="result-table-item"
                            onClick={() => handleCopyText(t, "表名")}
                            title="点击复制"
                          >
                            {t}
                            {Icons.copy}
                          </div>
                        ))}
                      </div>
                    </div>
                  </div>
                </>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Rule Modal */}
      {showModal && (
        <div className="modal-overlay" onClick={() => setShowModal(false)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h3>{editingRule ? "编辑规则" : "新建规则"}</h3>
              <button className="btn-close" onClick={() => setShowModal(false)}>
                {Icons.x}
              </button>
            </div>
            <div className="modal-body">
              <div className="form-section-title">
                {Icons.settings}
                基本信息
              </div>
              <div className="form-group">
                <label>
                  规则名称<span className="form-required">*</span>
                </label>
                <input
                  type="text"
                  placeholder="输入规则名称"
                  value={formData.name}
                  onInput={(e) => setFormData((p) => ({ ...p, name: (e.target as HTMLInputElement).value }))}
                />
              </div>
              <div className="form-group">
                <label>规则描述</label>
                <textarea
                  rows={2}
                  placeholder="输入规则描述（可选）"
                  value={formData.description}
                  onInput={(e) => setFormData((p) => ({ ...p, description: (e.target as HTMLInputElement).value }))}
                />
              </div>
              <div className="form-row">
                <div className="form-group">
                  <label>编号长度</label>
                  <input
                    type="text"
                    placeholder="0 = 任意"
                    value={formData.code_length}
                    onInput={(e) => setFormData((p) => ({ ...p, code_length: (e.target as HTMLInputElement).value }))}
                  />
                </div>
                <div className="form-group">
                  <label>编号前缀</label>
                  <input
                    type="text"
                    placeholder="多个用逗号分隔"
                    value={formData.code_prefix}
                    onInput={(e) => setFormData((p) => ({ ...p, code_prefix: (e.target as HTMLInputElement).value }))}
                  />
                </div>
              </div>
              <div className="form-group">
                <label>关联表名</label>
                <textarea
                  rows={3}
                  placeholder={"每行一个表名前缀，如：\nt_order\nt_order_item"}
                  value={formData.tables}
                  onInput={(e) => setFormData((p) => ({ ...p, tables: (e.target as HTMLInputElement).value }))}
                />
              </div>

              <div className="form-section-title">
                {Icons.fileCode}
                脚本配置
              </div>
              <div className="form-group">
                <div className="form-group-header">
                  <label>
                    解析脚本<span className="form-required">*</span>
                  </label>
                  {templates.length > 0 && (
                    <select
                      value=""
                      onChange={(e) => {
                        if (e.target.value) handleLoadTemplate(e.target.value);
                        e.target.value = "";
                      }}
                    >
                      <option value="">从模板加载</option>
                      {templates.map((t) => (
                        <option key={t.name} value={t.name}>
                          {t.name}
                        </option>
                      ))}
                    </select>
                  )}
                </div>
                <textarea
                  className="script-input"
                  rows={8}
                  placeholder={"Rhai 脚本，设置 database 和 table_suffix 变量\n例：\nlet database = \"db_\" + code[3..7];\nlet table_suffix = \"_\" + code[7..10];"}
                  value={formData.route_script}
                  onInput={(e) => setFormData((p) => ({ ...p, route_script: (e.target as HTMLInputElement).value }))}
                />
              </div>
            </div>
            <div className="modal-footer">
              <button className="btn btn-secondary" onClick={() => setShowModal(false)}>
                取消
              </button>
              <button className="btn btn-primary" onClick={handleSave}>
                保存
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Delete Confirm */}
      {showDeleteConfirm && (
        <div className="modal-overlay" onClick={() => setShowDeleteConfirm(null)}>
          <div className="modal modal-sm" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h3>确认删除</h3>
              <button className="btn-close" onClick={() => setShowDeleteConfirm(null)}>
                {Icons.x}
              </button>
            </div>
            <div className="modal-body">
              <p>确定要删除此规则吗？此操作不可撤销。</p>
            </div>
            <div className="modal-footer">
              <button className="btn btn-secondary" onClick={() => setShowDeleteConfirm(null)}>
                取消
              </button>
              <button className="btn btn-danger" onClick={() => handleDelete(showDeleteConfirm)}>
                删除
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
