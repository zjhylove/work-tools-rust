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

// --- Helpers ---

const call = async (method: string, params: Record<string, unknown> = {}) => {
  return await window.pluginAPI?.call("db-router", method, params);
};

// --- App ---

function App() {
  const [rules, setRules] = useState<RouteRule[]>([]);
  const [code, setCode] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [result, setResult] = useState<RouteResult | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);

  // Modal state
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

  // Templates
  const [templates, setTemplates] = useState<RouteRule[]>([]);

  // Toast helper
  const addToast = useCallback((message: string, type: Toast["type"] = "info") => {
    const id = Date.now();
    setToasts((prev) => [...prev, { message, type, id }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 3000);
  }, []);

  // Load data
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

  // Filter rules by code (matching) AND search query (name)
  const filteredRules = useMemo(() => {
    let filtered = rules;

    // Filter by code matching (if code is entered)
    if (code.trim()) {
      filtered = filtered.filter((r) => {
        if (r.code_length > 0 && code.length !== r.code_length) return false;
        if (r.code_prefix) {
          const prefixes = r.code_prefix.split(",").map((s) => s.trim());
          if (prefixes.length > 0 && !prefixes.some((p) => code.startsWith(p))) return false;
        }
        return true;
      });
    }

    // Filter by search query (name)
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      filtered = filtered.filter((r) => r.name.toLowerCase().includes(q));
    }

    return filtered;
  }, [rules, code, searchQuery]);

  // Parse route
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

  // CRUD
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

  // Import / Export
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
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `db-router-rules-${new Date().toISOString().split("T")[0]}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      addToast("规则已导出", "success");
    } catch {
      addToast("导出失败", "error");
    }
  };

  // Copy result
  const handleCopyResult = async () => {
    if (!result) return;
    const text = `database: ${result.database}\ntables:\n${result.tables.map((t) => `  - ${t}`).join("\n")}`;
    try {
      await navigator.clipboard.writeText(text);
      addToast("已复制到剪贴板", "success");
    } catch {
      addToast("复制失败", "error");
    }
  };

  // Template select
  const handleLoadTemplate = (templateName: string) => {
    const template = templates.find((t) => t.name === templateName);
    if (template) {
      setFormData((prev) => ({
        ...prev,
        route_script: template.route_script,
      }));
    }
  };

  return (
    <div className="db-router">
      {/* Toasts */}
      {toasts.map((t) => (
        <div key={t.id} className={`toast toast-${t.type}`}>
          {t.message}
        </div>
      ))}

      {/* Toolbar */}
      <div className="toolbar">
        <div className="toolbar-title">🔍 数据库路由</div>
        <div className="toolbar-actions">
          <button className="btn btn-primary" onClick={handleOpenNew}>+ 新建规则</button>
          <button className="btn btn-secondary" onClick={handleImport}>📥 导入</button>
          <button className="btn btn-secondary" onClick={handleExport}>📤 导出</button>
        </div>
      </div>

      {/* Main Content */}
      <div className="main-content">
        {/* Left Panel: Rules */}
        <div className="left-panel">
          <div className="panel-search">
            <input
              type="text"
              className="search-input"
              placeholder="🔍 搜索规则..."
              value={searchQuery}
              onInput={(e) => setSearchQuery((e.target as HTMLInputElement).value)}
            />
          </div>
          <div className="rule-list">
            {filteredRules.length === 0 ? (
              <div className="empty-state">
                <div className="empty-icon">📭</div>
                <div className="empty-text">
                  {code || searchQuery ? "没有匹配的规则" : "还没有路由规则"}
                </div>
              </div>
            ) : (
              filteredRules.map((rule) => (
                <div key={rule.id} className="rule-card">
                  <div className="rule-info">
                    <div className="rule-name">{rule.name}</div>
                    {rule.description && <div className="rule-desc">{rule.description}</div>}
                    <div className="rule-tags">
                      <span className="tag">
                        {rule.code_length > 0 ? `长度: ${rule.code_length}` : "长度: 任意"}
                      </span>
                      <span className="tag">
                        {rule.code_prefix ? `前缀: ${rule.code_prefix}` : "前缀: 任意"}
                      </span>
                      {rule.tables.length > 0 && (
                        <span className="tag tag-tables">关联 {rule.tables.length} 张表</span>
                      )}
                    </div>
                  </div>
                  <div className="rule-actions">
                    <button className="btn-icon" onClick={() => handleParse(rule.id)} title="解析">▶️</button>
                    <button className="btn-icon" onClick={() => handleOpenEdit(rule)} title="编辑">✏️</button>
                    <button className="btn-icon btn-icon-danger" onClick={() => setShowDeleteConfirm(rule.id)} title="删除">🗑️</button>
                  </div>
                </div>
              ))
            )}
          </div>
          <div className="panel-footer">共 {rules.length} 条规则</div>
        </div>

        {/* Right Panel: Parser */}
        <div className="right-panel">
          <div className="parser-section">
            <label className="section-label">输入编号</label>
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
            <div className="section-hint">输入编号后点击规则卡片上的 ▶ 执行解析</div>
          </div>

          <div className="parser-section result-section">
            <label className="section-label">解析结果</label>
            <div className="result-card">
              {!result ? (
                <div className="result-empty">
                  <div className="empty-icon">🔍</div>
                  <div className="empty-text">输入编号并选择规则进行解析</div>
                </div>
              ) : (
                <>
                  <div className="result-header">
                    <span className="result-success">✅ 解析成功</span>
                    <span className="result-rule">使用规则: {result.rule_name}</span>
                  </div>
                  <div className="result-body">
                    <div className="result-field">
                      <div className="result-field-label">数据库 (database)</div>
                      <div className="result-field-value">{result.database}</div>
                    </div>
                    <div className="result-field">
                      <div className="result-field-label">
                        数据表 (tables)
                        {result.tables.length > 0 && <span className="table-count"> {result.tables.length} 张</span>}
                      </div>
                      <div className="result-tables">
                        {result.tables.map((t, i) => (
                          <div key={i} className="result-table-item">{t}</div>
                        ))}
                      </div>
                    </div>
                  </div>
                </>
              )}
            </div>
          </div>

          <div className="parser-actions">
            {result && (
              <button className="btn btn-secondary" onClick={handleCopyResult}>📋 复制结果</button>
            )}
          </div>
        </div>
      </div>

      {/* Rule Modal */}
      {showModal && (
        <div className="modal-overlay" onClick={() => setShowModal(false)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h3>{editingRule ? "编辑规则" : "新建规则"}</h3>
              <button className="btn-close" onClick={() => setShowModal(false)}>✕</button>
            </div>
            <div className="modal-body">
              <div className="form-group">
                <label>规则名称 *</label>
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
              <div className="form-group">
                <div className="form-group-header">
                  <label>解析脚本 *</label>
                  {templates.length > 0 && (
                    <select
                      value=""
                      onChange={(e) => {
                        if (e.target.value) handleLoadTemplate(e.target.value);
                        e.target.value = "";
                      }}
                    >
                      <option value="">💡 从模板加载</option>
                      {templates.map((t) => (
                        <option key={t.name} value={t.name}>{t.name}</option>
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
              <button className="btn btn-secondary" onClick={() => setShowModal(false)}>取消</button>
              <button className="btn btn-primary" onClick={handleSave}>保存</button>
            </div>
          </div>
        </div>
      )}

      {/* Delete Confirm */}
      {showDeleteConfirm && (
        <div className="modal-overlay" onClick={() => setShowDeleteConfirm(null)}>
          <div className="modal modal-sm" onClick={(e) => e.stopPropagation()}>
            <h3>确认删除</h3>
            <p>确定要删除此规则吗？此操作不可撤销。</p>
            <div className="modal-footer">
              <button className="btn btn-secondary" onClick={() => setShowDeleteConfirm(null)}>取消</button>
              <button className="btn btn-danger" onClick={() => handleDelete(showDeleteConfirm)}>删除</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
