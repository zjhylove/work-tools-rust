import { useState, useEffect, useCallback, useMemo } from 'react';
import './App.css';

declare global {
  interface Window {
    pluginAPI?: {
      call: (pluginId: string, method: string, params?: Record<string, unknown>) => Promise<unknown>;
    };
  }
}

interface KeyInfo { key: string; type: string; ttl: number; }
interface SavedConn { id: string; name: string; host: string; port: number; db: number; has_password: boolean; }
interface TreeNode { name: string; fullKey: string | null; keyInfo?: KeyInfo; children: TreeNode[]; }

function buildTree(keys: KeyInfo[]): TreeNode[] {
  const root: TreeNode = { name: '', fullKey: null, children: [] };
  for (const k of keys) {
    const parts = k.key.split(':');
    let node = root;
    for (let i = 0; i < parts.length; i++) {
      const isLast = i === parts.length - 1;
      let child = node.children.find(c => c.name === parts[i]);
      if (!child) {
        child = { name: parts[i], fullKey: isLast ? k.key : null, children: [] };
        if (isLast) child.keyInfo = k;
        node.children.push(child);
      } else if (isLast) {
        child.fullKey = k.key;
        child.keyInfo = k;
      }
      node = child;
    }
  }
  return root.children;
}

const STORAGE_KEY = 'redis_client_last_conn';

function loadLastConn() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw) as { host: string; port: number; db: number };
  } catch { /* ignore */ }
  return { host: '127.0.0.1', port: 6379, db: 0 };
}

function saveLastConn(host: string, port: number, db: number) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ host, port, db }));
  } catch { /* ignore */ }
}

function TreeItem({ node, depth, selectedKey, expandedPaths, onToggle, onSelect }: {
  node: TreeNode; depth: number; selectedKey: string | null;
  expandedPaths: Set<string>; onToggle: (p: string) => void; onSelect: (k: string) => void;
}) {
  const path = node.fullKey || node.name;
  const isFolder = node.fullKey === null;
  const isExpanded = expandedPaths.has(path);

  if (isFolder) {
    return (
      <div className="tree-branch">
        <div
          className="tree-folder"
          style={{ paddingLeft: depth * 14 + 8 }}
          onClick={() => onToggle(path)}
        >
          <span className="tree-arrow">{isExpanded ? '▾' : '▸'}</span>
          <span className="tree-folder-name">{node.name}</span>
        </div>
        {isExpanded && node.children.map(child => (
          <TreeItem key={child.name} node={child} depth={depth + 1}
            selectedKey={selectedKey} expandedPaths={expandedPaths}
            onToggle={onToggle} onSelect={onSelect} />
        ))}
      </div>
    );
  }

  return (
    <div
      className={`tree-leaf ${selectedKey === node.fullKey ? 'selected' : ''}`}
      style={{ paddingLeft: depth * 14 + 24 }}
      onClick={() => node.fullKey && onSelect(node.fullKey)}
    >
      {node.keyInfo && (
        <span className="key-type-badge" data-type={node.keyInfo.type}>{node.keyInfo.type}</span>
      )}
      <span className="tree-leaf-name">{node.name}</span>
      {node.keyInfo && node.keyInfo.ttl > 0 && (
        <span className="key-ttl">{node.keyInfo.ttl}s</span>
      )}
    </div>
  );
}

function App() {
  const [connected, setConnected] = useState(false);
  const [connForm, setConnForm] = useState(() => {
    const last = loadLastConn();
    return { host: last.host, port: last.port, db: last.db, password: '' };
  });
  const [savedConns, setSavedConns] = useState<SavedConn[]>([]);
  const [keys, setKeys] = useState<KeyInfo[]>([]);
  const [search, setSearch] = useState('*');
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [keyDetail, setKeyDetail] = useState<Record<string, unknown> | null>(null);
  const [valueData, setValueData] = useState<unknown>(null);
  const [error, setError] = useState('');
  const [editingField, setEditingField] = useState({ field: '', value: '' });
  const [editingStringValue, setEditingStringValue] = useState('');
  const [saveConnName, setSaveConnName] = useState('');
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [scanLoading, setScanLoading] = useState(false);
  const [detailLoading, setDetailLoading] = useState(false);
  const [hasScanned, setHasScanned] = useState(false);
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());

  const tree = useMemo(() => buildTree(keys), [keys]);

  const togglePath = useCallback((path: string) => {
    setExpandedPaths(prev => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path); else next.add(path);
      return next;
    });
  }, []);

  const clearError = () => setError('');
  const showError = (e: unknown) => setError(String(e));

  const loadSavedConns = useCallback(async () => {
    try {
      const r = await window.pluginAPI?.call('redis-client', 'list_saved_connections', {});
      if (r && typeof r === 'object' && 'connections' in r) {
        setSavedConns((r as { connections: SavedConn[] }).connections);
      }
    } catch { /* ignore */ }
  }, []);

  useEffect(() => { loadSavedConns(); }, [loadSavedConns]);

  const handleConnect = useCallback(async (host: string, port: number, db: number, password: string) => {
    clearError();
    try {
      await window.pluginAPI?.call('redis-client', 'connect', { host, port, db, password });
      setConnected(true);
      saveLastConn(host, port, db);
      setScanLoading(true);
      setHasScanned(false);
      const r = await window.pluginAPI?.call('redis-client', 'scan_keys', { cursor: 0, pattern: '*', count: 50 });
      if (r && typeof r === 'object' && 'keys' in r) {
        setKeys((r as { keys: KeyInfo[] }).keys);
      }
      setHasScanned(true);
    } catch (e) { showError(e); }
    finally { setScanLoading(false); }
  }, []);

  const handleDisconnect = useCallback(async () => {
    await window.pluginAPI?.call('redis-client', 'disconnect', {});
    setConnected(false);
    setKeys([]);
    setSelectedKey(null);
    setKeyDetail(null);
    setValueData(null);
    // 恢复上次连接的表单（密码不清）
    const last = loadLastConn();
    setConnForm({ host: last.host, port: last.port, db: last.db, password: '' });
    loadSavedConns();
  }, [loadSavedConns]);

  const handleSaveCurrentConn = useCallback(async () => {
    const name = saveConnName.trim();
    if (!name) return;
    try {
      const last = loadLastConn();
      await window.pluginAPI?.call('redis-client', 'save_connection', {
        name,
        host: last.host,
        port: last.port,
        db: last.db,
        password: connForm.password,
      });
      setSaveConnName('');
      setShowSaveDialog(false);
      loadSavedConns();
      setError(`连接 "${name}" 已保存`);
    } catch (e) { showError(e); }
  }, [saveConnName, connForm.password, loadSavedConns]);

  const handleDeleteSavedConn = useCallback(async (id: string) => {
    try {
      await window.pluginAPI?.call('redis-client', 'delete_saved_connection', { id });
      loadSavedConns();
    } catch (e) { showError(e); }
  }, [loadSavedConns]);

  const handleScan = useCallback(async (pattern?: string) => {
    setScanLoading(true);
    setHasScanned(false);
    try {
      const r = await window.pluginAPI?.call('redis-client', 'scan_keys', { cursor: 0, pattern: pattern || search, count: 50 });
      if (r && typeof r === 'object' && 'keys' in r) {
        setKeys((r as { keys: KeyInfo[] }).keys);
      }
      setHasScanned(true);
    } catch (e) { showError(e); }
    finally { setScanLoading(false); }
  }, [search]);

  const handleSelectKey = useCallback(async (key: string) => {
    setSelectedKey(key); clearError();
    setDetailLoading(true);
    setValueData(null);
    try {
      const info = await window.pluginAPI?.call('redis-client', 'get_key_info', { key });
      setKeyDetail(info as Record<string, unknown>);
      const kType = (info as Record<string, string>).type;
      if (kType === 'string') {
        const v = await window.pluginAPI?.call('redis-client', 'get_string', { key });
        setValueData(v);
      } else if (kType === 'hash') {
        const v = await window.pluginAPI?.call('redis-client', 'get_hash', { key });
        setValueData(v);
      } else if (kType === 'list') {
        const v = await window.pluginAPI?.call('redis-client', 'get_list', { key, start: 0, stop: -1 });
        setValueData(v);
      } else if (kType === 'set') {
        const v = await window.pluginAPI?.call('redis-client', 'get_set', { key });
        setValueData(v);
      } else if (kType === 'zset') {
        const v = await window.pluginAPI?.call('redis-client', 'get_zset', { key });
        setValueData(v);
      }
    } catch (e) { showError(e); }
    finally { setDetailLoading(false); }
  }, []);

  const handleDeleteKey = useCallback(async () => {
    if (!selectedKey) return;
    try {
      await window.pluginAPI?.call('redis-client', 'delete_key', { key: selectedKey });
      setSelectedKey(null); setKeyDetail(null); setValueData(null);
      handleScan();
    } catch (e) { showError(e); }
  }, [selectedKey, handleScan]);

  const handleSetHashField = useCallback(async (field: string, value: string) => {
    if (!selectedKey) return;
    try {
      await window.pluginAPI?.call('redis-client', 'set_hash_field', { key: selectedKey, field, value });
      setEditingField({ field: '', value: '' });
      handleSelectKey(selectedKey);
    } catch (e) { showError(e); }
  }, [selectedKey, handleSelectKey]);

  const handleDelHashField = useCallback(async (field: string) => {
    if (!selectedKey) return;
    try {
      await window.pluginAPI?.call('redis-client', 'del_hash_field', { key: selectedKey, field });
      handleSelectKey(selectedKey);
    } catch (e) { showError(e); }
  }, [selectedKey, handleSelectKey]);

  const handleSaveString = useCallback(async () => {
    if (!selectedKey) return;
    try {
      await window.pluginAPI?.call('redis-client', 'set_string', { key: selectedKey, value: editingStringValue });
      handleSelectKey(selectedKey);
    } catch (e) { showError(e); }
  }, [selectedKey, editingStringValue, handleSelectKey]);

  return (
    <div className="redis-client">
      {!connected ? (
        <div className="connect-panel">
          <h3>连接 Redis</h3>
          <div className="form-group">
            <label>Host</label>
            <input type="text" value={connForm.host} onChange={e => setConnForm(p => ({ ...p, host: e.target.value }))} />
          </div>
          <div className="form-group">
            <label>Port</label>
            <input type="number" value={connForm.port} onChange={e => setConnForm(p => ({ ...p, port: Number(e.target.value) }))} />
          </div>
          <div className="form-group">
            <label>DB</label>
            <input type="number" value={connForm.db} onChange={e => setConnForm(p => ({ ...p, db: Number(e.target.value) }))} />
          </div>
          <div className="form-group">
            <label>Password</label>
            <input type="password" value={connForm.password} onChange={e => setConnForm(p => ({ ...p, password: e.target.value }))}
              onKeyDown={e => e.key === 'Enter' && handleConnect(connForm.host, connForm.port, connForm.db, connForm.password)} />
          </div>
          <button className="btn-primary" onClick={() => handleConnect(connForm.host, connForm.port, connForm.db, connForm.password)}>连接</button>

          {savedConns.length > 0 && (
            <div className="saved-connections">
              <h4>已保存连接</h4>
              {savedConns.map(c => (
                <div key={c.id} className="saved-conn-item">
                  <div className="saved-conn-main" onClick={async () => {
                    try {
                      const r = await window.pluginAPI?.call('redis-client', 'get_saved_password', { id: c.id });
                      const pass = (r as { password: string }).password || '';
                      handleConnect(c.host, c.port, c.db, pass);
                    } catch (e) { showError(e); }
                  }}>
                    <span className="conn-name">{c.name}</span>
                    <span className="conn-info">{c.host}:{c.port} db{c.db}</span>
                  </div>
                  <button className="btn-conn-delete" onClick={e => { e.stopPropagation(); handleDeleteSavedConn(c.id); }} title="删除">✕</button>
                </div>
              ))}
            </div>
          )}
        </div>
      ) : (
        <div className="main-layout">
          <div className="key-panel">
            <div className="panel-header">
              <span className="status-dot" title="已连接" />
              <input type="text" value={search} onChange={e => setSearch(e.target.value)}
                placeholder="搜索 key (* 通配)" onKeyDown={e => e.key === 'Enter' && handleScan()} />
              <button onClick={() => handleScan()}>🔍</button>
              <button onClick={handleDisconnect} title="断开连接">✕</button>
            </div>
            <div className="key-list">
              {scanLoading ? (
                <div className="list-status"><span className="spinner" />扫描中…</div>
              ) : tree.length > 0 ? (
                tree.map(node => (
                  <TreeItem
                    key={node.name}
                    node={node}
                    depth={0}
                    selectedKey={selectedKey}
                    expandedPaths={expandedPaths}
                    onToggle={togglePath}
                    onSelect={handleSelectKey}
                  />
                ))
              ) : hasScanned ? (
                <div className="list-status">无匹配的 Key</div>
              ) : (
                <div className="list-status">输入 pattern 后搜索</div>
              )}
            </div>
          </div>

          <div className="detail-panel">
            {detailLoading ? (
              <div className="detail-loading"><span className="spinner" />加载中…</div>
            ) : selectedKey && keyDetail ? (
              <div className="detail-content">
                <div className="detail-header">
                  <h4>{selectedKey}</h4>
                  <span className="type-badge">{keyDetail.type as string}</span>
                  <span className="ttl-badge">TTL: {keyDetail.ttl as number}s</span>
                  <button className="btn-danger" onClick={handleDeleteKey}>删除</button>
                </div>

                {keyDetail.type === 'string' && !!valueData && (
                  <div className="value-editor">
                    <textarea
                      value={editingStringValue || (valueData as { value: string }).value}
                      onChange={e => setEditingStringValue(e.target.value)}
                      onFocus={() => setEditingStringValue((valueData as { value: string }).value)}
                      rows={12}
                    />
                    <button className="btn-primary" onClick={handleSaveString}>保存</button>
                  </div>
                )}

                {keyDetail.type === 'hash' && !!valueData && (
                  <div className="hash-editor">
                    <table>
                      <thead><tr><th>Field</th><th>Value</th><th>操作</th></tr></thead>
                      <tbody>
                        {Object.entries((valueData as { fields: Record<string, string> }).fields).map(([f, v]) => (
                          <tr key={f}><td><code>{f}</code></td><td><code>{v}</code></td>
                            <td><button onClick={() => handleDelHashField(f)}>删除</button></td></tr>
                        ))}
                      </tbody>
                    </table>
                    <div className="add-field">
                      <input placeholder="field" value={editingField.field} onChange={e => setEditingField(p => ({ ...p, field: e.target.value }))} />
                      <input placeholder="value" value={editingField.value} onChange={e => setEditingField(p => ({ ...p, value: e.target.value }))} />
                      <button className="btn-primary" onClick={() => handleSetHashField(editingField.field, editingField.value)}>添加</button>
                    </div>
                  </div>
                )}

                {keyDetail.type === 'list' && !!valueData && (
                  <div className="list-editor">
                    <ol>{(valueData as { items: string[] }).items.map((item, i) => <li key={i}><code>{item}</code></li>)}</ol>
                  </div>
                )}

                {keyDetail.type === 'set' && !!valueData && (
                  <div className="set-editor">
                    {(valueData as { members: string[] }).members.map(m => <span key={m} className="member-tag">{m}</span>)}
                  </div>
                )}

                {keyDetail.type === 'zset' && !!valueData && (
                  <div className="zset-editor">
                    <table>
                      <thead><tr><th>Member</th><th>Score</th></tr></thead>
                      <tbody>
                        {(valueData as { members: Array<{ member: string; score: number }> }).members.map(m => (
                          <tr key={m.member}><td><code>{m.member}</code></td><td>{m.score}</td></tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                )}

                {/* 保存当前连接 */}
                <div className="save-conn-section">
                  {!showSaveDialog ? (
                    <button className="btn-secondary" onClick={() => setShowSaveDialog(true)}>💾 保存此连接</button>
                  ) : (
                    <div className="save-conn-dialog">
                      <input
                        type="text"
                        placeholder="连接名称，如 dev / staging"
                        value={saveConnName}
                        onChange={e => setSaveConnName(e.target.value)}
                        onKeyDown={e => e.key === 'Enter' && handleSaveCurrentConn()}
                      />
                      <button className="btn-primary" onClick={handleSaveCurrentConn}>保存</button>
                      <button className="btn-secondary" onClick={() => { setShowSaveDialog(false); setSaveConnName(''); }}>取消</button>
                    </div>
                  )}
                </div>
              </div>
            ) : (
              <div className="empty-detail">选择一个 Key 查看详情</div>
            )}
          </div>
        </div>
      )}

      {error && <div className="error-toast" onClick={() => setError('')}>{error} (点击关闭)</div>}
    </div>
  );
}

export default App;
