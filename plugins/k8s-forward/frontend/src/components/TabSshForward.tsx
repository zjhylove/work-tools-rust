import { useState, useEffect, useCallback } from "react";
import type { ForwardRule, SshConnection, SshStatus } from "../types";

declare global {
  interface Window { WorkTools: { toast: { success(m:string):void; error(m:string):void; info(m:string):void; warning(m:string):void }; FieldError: { show(el:HTMLElement, m:string):void; clear(el:HTMLElement):void; clearAll(f:HTMLElement):void } } }
}

const PLUGIN_ID = "k8s-forward";

export default function TabSshForward() {
  const [connections, setConnections] = useState<SshStatus[]>([]);
  const [rules, setRules] = useState<ForwardRule[]>([]);
  const [selectedConnId, setSelectedConnId] = useState<string>("");
  const [editingRule, setEditingRule] = useState<ForwardRule | null>(null);
  const [isNewRule, setIsNewRule] = useState(false);
  const [editingConn, setEditingConn] = useState<{ id?: string; name: string; host: string; port: number; username: string; password: string } | null>(null);

  const call = useCallback(async (method: string, params?: unknown) => {
    return await window.pluginAPI.call(PLUGIN_ID, method, (params ?? {}) as Record<string, unknown>);
  }, []);

  const loadConnections = async () => {
    const list = await call("ssh_list_connections") as SshStatus[];
    setConnections(list);
  };

  const loadRules = async () => {
    const r = await call("list_forward_rules") as ForwardRule[];
    setRules(r.filter(r => r.rule_type === "Manual"));
  };

  useEffect(() => {
    Promise.allSettled([loadConnections(), loadRules()]).then(() => {});
  }, []);

  useEffect(() => {
    const hasReconnecting = connections.some(c => c.status === "Reconnecting");
    if (!hasReconnecting) return;
    const timer = setInterval(async () => {
      const list = await call("ssh_list_connections") as SshStatus[];
      setConnections(list);
    }, 5000);
    return () => clearInterval(timer);
  }, [connections]);

  const handleConnect = async (connectionId: string) => {
    try {
      await call("ssh_connect", { connection_id: connectionId });
      window.WorkTools.toast.success("SSH 连接成功");
      loadConnections();
    } catch (e: unknown) { window.WorkTools.toast.error(`连接失败: ${e}`); }
  };

  const handleDisconnect = async (connectionId: string) => {
    try {
      await call("ssh_disconnect", { connection_id: connectionId });
      window.WorkTools.toast.info("SSH 已断开");
      loadConnections();
    } catch (e: unknown) { window.WorkTools.toast.error(`断开失败: ${e}`); }
  };

  const handleSaveConnection = async () => {
    if (!editingConn) return;
    try {
      if (editingConn.id) {
        await call("ssh_update_connection", editingConn);
        window.WorkTools.toast.success("连接已更新");
      } else {
        await call("ssh_add_connection", editingConn);
        window.WorkTools.toast.success("连接已添加");
      }
      setEditingConn(null);
      loadConnections();
    } catch (e: unknown) { window.WorkTools.toast.error(`保存失败: ${e}`); }
  };

  const handleRemoveConnection = async (id: string) => {
    try {
      const result = await call("ssh_remove_connection", { id }) as { removed_rules: number };
      if (result.removed_rules > 0) {
        window.WorkTools.toast.info(`已删除连接及 ${result.removed_rules} 条关联规则`);
      } else {
        window.WorkTools.toast.success("连接已删除");
      }
      if (selectedConnId === id) setSelectedConnId("");
      loadConnections();
      loadRules();
    } catch (e: unknown) { window.WorkTools.toast.error(`删除失败: ${e}`); }
  };

  const handleAddRule = () => {
    if (!selectedConnId && connections.length > 0) {
      window.WorkTools.toast.warning("请先选择一个 SSH 连接");
      return;
    }
    const rule: ForwardRule = {
      id: window.crypto.randomUUID(),
      name: `rule-${Date.now()}`,
      local_host: "127.0.0.1",
      local_port: 0,
      remote_host: "",
      remote_port: 0,
      rule_type: "Manual" as const,
      ssh_connection_id: selectedConnId,
    };
    setEditingRule(rule);
    setIsNewRule(true);
  };

  const handleSaveRule = async () => {
    if (!editingRule) return;
    try {
      if (isNewRule) {
        await call("add_forward_rule", editingRule);
      } else {
        await call("update_forward_rule", editingRule);
      }
      window.WorkTools.toast.success(isNewRule ? "规则已添加" : "已保存");
      setEditingRule(null);
      setIsNewRule(false);
      loadRules();
    } catch (e: unknown) { window.WorkTools.toast.error(`保存失败: ${e}`); }
  };

  const handleDeleteRule = async (id: string) => {
    try {
      await call("remove_forward_rule", { id });
      window.WorkTools.toast.success("已删除");
      loadRules();
    } catch (e: unknown) { window.WorkTools.toast.error(`删除失败: ${e}`); }
  };

  const handleImport = () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json";
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      try {
        const text = await file.text();
        const parsed = JSON.parse(text);
        const arr = Array.isArray(parsed) ? parsed : parsed.rules || [];
        await call("import_rules", { rules: arr });
        window.WorkTools.toast.success(`已导入 ${arr.length} 条规则`);
        loadRules();
      } catch { window.WorkTools.toast.error("导入失败: 格式错误"); }
    };
    input.click();
  };

  const handleExport = async () => {
    try {
      const dir = await window.pluginAPI.open_folder_dialog("选择导出目录");
      if (!dir) return;
      const data = await call("export_rules") as ForwardRule[];
      const json = JSON.stringify(data.filter(r => r.rule_type === "Manual"), null, 2);
      const filename = `k8s-forward-rules-${new Date().toISOString().split("T")[0]}.json`;
      const filePath = `${dir.replace(/\\/g, "/")}/${filename}`;
      await window.pluginAPI.write_file(filePath, json);
      window.WorkTools.toast.success(`已导出到 ${filePath}`);
    } catch (e: unknown) { window.WorkTools.toast.error(`导出失败: ${e}`); }
  };

  const getConnectionName = (connId: string) => {
    const conn = connections.find(c => c.connection_id === connId);
    return conn?.connection_name || connId;
  };

  const filteredRules = selectedConnId
    ? rules.filter(r => r.ssh_connection_id === selectedConnId)
    : rules;

  return (
    <div>
      <div className="card">
        <div className="card-header" style={{display:"flex",justifyContent:"space-between",alignItems:"center"}}>
          <span>SSH 连接管理</span>
          <button className="btn btn-primary btn-sm" onClick={() => setEditingConn({ id: undefined, name: "", host: "", port: 22, username: "", password: "" })}>+ 添加 SSH</button>
        </div>
        {connections.length === 0 ? (
          <div style={{textAlign:"center",color:"var(--text-tertiary)",padding:20}}>暂无 SSH 连接，点击右上角添加</div>
        ) : (
          <table>
            <thead><tr><th>名称</th><th>地址</th><th>状态</th><th>操作</th></tr></thead>
            <tbody>
              {connections.map(c => (
                <tr key={c.connection_id}>
                  <td>{c.connection_name}</td>
                  <td><code>{c.host}:{c.port}</code></td>
                  <td>
                    <span className={`status-dot ${
                      c.status === "Connected" ? "online" :
                      c.status === "Reconnecting" ? "reconnecting" :
                      "offline"
                    }`}></span>
                    {c.status === "Connected" && "已连接"}
                    {c.status === "Reconnecting" && `重连中 (${c.reconnect_info?.retry_count ?? 0}/${c.reconnect_info?.max_retries ?? 10})`}
                    {c.status === "Disconnected" && "未连接"}
                  </td>
                  <td style={{whiteSpace:"nowrap"}}>
                    {c.status === "Connected" ? (
                      <button className="btn btn-danger btn-sm" onClick={() => handleDisconnect(c.connection_id!)}>断开</button>
                    ) : c.status === "Reconnecting" ? (
                      <button className="btn btn-secondary btn-sm" disabled>重连中...</button>
                    ) : (
                      <button className="btn btn-primary btn-sm" onClick={() => handleConnect(c.connection_id!)}>连接</button>
                    )}
                    <button className="btn btn-secondary btn-sm" style={{marginLeft:4}} onClick={() => setEditingConn({ id: c.connection_id!, name: c.connection_name || "", host: c.host || "", port: c.port || 22, username: "", password: "" })}>编辑</button>
                    <button className="btn btn-danger btn-sm" style={{marginLeft:4}} onClick={() => handleRemoveConnection(c.connection_id!)}>删除</button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <div className="card">
        <div className="card-header" style={{display:"flex",justifyContent:"space-between",alignItems:"center"}}>
          <span style={{display:"flex",alignItems:"center",gap:8}}>
            转发规则
            <select value={selectedConnId} onChange={e => setSelectedConnId(e.target.value)} style={{fontSize:12,padding:"2px 4px"}}>
              <option value="">全部连接</option>
              {connections.map(c => (
                <option key={c.connection_id} value={c.connection_id}>{c.connection_name}</option>
              ))}
            </select>
          </span>
          <div style={{display:"flex",gap:8}}>
            <button className="btn btn-primary btn-sm" onClick={handleAddRule}>+ 添加规则</button>
            <button className="btn btn-secondary btn-sm" onClick={handleImport}>导入</button>
            <button className="btn btn-secondary btn-sm" onClick={handleExport}>导出</button>
          </div>
        </div>
        <table>
          <thead>
            <tr>
              <th>名称</th>
              <th>本地地址</th>
              <th>本地端口</th>
              <th>远程地址</th>
              <th>远程端口</th>
              {!selectedConnId && <th>SSH连接</th>}
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            {filteredRules.map(r => (
              <tr key={r.id}>
                <td>{r.name}</td>
                <td>{r.local_host}</td>
                <td>{r.local_port}</td>
                <td>{r.remote_host}</td>
                <td>{r.remote_port}</td>
                {!selectedConnId && <td>{getConnectionName(r.ssh_connection_id)}</td>}
                <td>
                  <button className="btn btn-secondary btn-sm" onClick={() => { setEditingRule(r); setIsNewRule(false); }} style={{marginRight:4}}>编辑</button>
                  <button className="btn btn-danger btn-sm" onClick={() => handleDeleteRule(r.id)}>删除</button>
                </td>
              </tr>
            ))}
            {filteredRules.length === 0 && <tr><td colSpan={selectedConnId ? 6 : 7} style={{textAlign:"center",color:"var(--text-tertiary)",padding:20}}>暂无规则</td></tr>}
          </tbody>
        </table>
      </div>

      {editingConn && (
        <div className="modal-overlay" onClick={() => setEditingConn(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>{editingConn.id ? "编辑 SSH 连接" : "添加 SSH 连接"}</h3>
            <div className="form-row">
              <div className="form-group"><label>名称</label><input value={editingConn.name} onChange={e => setEditingConn({...editingConn, name: e.target.value})} placeholder="如：生产环境" /></div>
              <div className="form-group"><label>主机地址</label><input value={editingConn.host} onChange={e => setEditingConn({...editingConn, host: e.target.value})} placeholder="10.73.x.x" /></div>
              <div className="form-group"><label>端口</label><input type="number" value={editingConn.port} onChange={e => setEditingConn({...editingConn, port: +e.target.value})} /></div>
            </div>
            <div className="form-row">
              <div className="form-group"><label>用户名</label><input value={editingConn.username} onChange={e => setEditingConn({...editingConn, username: e.target.value})} /></div>
              <div className="form-group"><label>密码</label><input type="password" value={editingConn.password} onChange={e => setEditingConn({...editingConn, password: e.target.value})} placeholder={editingConn.id ? "留空则不修改" : ""} /></div>
            </div>
            <div className="modal-actions">
              <button className="btn btn-secondary" onClick={() => setEditingConn(null)}>取消</button>
              <button className="btn btn-primary" onClick={handleSaveConnection}>保存</button>
            </div>
          </div>
        </div>
      )}

      {editingRule && (
        <div className="modal-overlay" onClick={() => setEditingRule(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>编辑规则</h3>
            <div className="form-row">
              <div className="form-group"><label>名称</label><input value={editingRule.name} onChange={e => setEditingRule({...editingRule, name: e.target.value})} /></div>
              <div className="form-group"><label>本地地址</label><input value={editingRule.local_host} onChange={e => setEditingRule({...editingRule, local_host: e.target.value})} /></div>
              <div className="form-group"><label>本地端口</label><input type="number" value={editingRule.local_port} onChange={e => setEditingRule({...editingRule, local_port: +e.target.value})} /></div>
              <div className="form-group"><label>远程地址</label><input value={editingRule.remote_host} onChange={e => setEditingRule({...editingRule, remote_host: e.target.value})} /></div>
              <div className="form-group"><label>远程端口</label><input type="number" value={editingRule.remote_port} onChange={e => setEditingRule({...editingRule, remote_port: +e.target.value})} /></div>
            </div>
            <div className="modal-actions">
              <button className="btn btn-secondary" onClick={() => { setEditingRule(null); setIsNewRule(false); }}>取消</button>
              <button className="btn btn-primary" onClick={handleSaveRule}>保存</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
