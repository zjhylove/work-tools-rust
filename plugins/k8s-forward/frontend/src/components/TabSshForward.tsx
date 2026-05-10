import { useState, useEffect, useCallback } from "react";
import type { ForwardRule, SshStatus } from "../types";

declare global {
  interface Window { WorkTools: { toast: { success(m:string):void; error(m:string):void; info(m:string):void; warning(m:string):void }; FieldError: { show(el:HTMLElement, m:string):void; clear(el:HTMLElement):void; clearAll(f:HTMLElement):void } } }
}

const PLUGIN_ID = "k8s-forward";

export default function TabSshForward() {
  const [sshStatus, setSshStatus] = useState<SshStatus>({ connected: false });
  const [rules, setRules] = useState<ForwardRule[]>([]);
  const [form, setForm] = useState({ host: "", port: 22, username: "", password: "" });
  const [editing, setEditing] = useState<ForwardRule | null>(null);
  const [isNewRule, setIsNewRule] = useState(false);

  const call = useCallback(async (method: string, params?: unknown) => {
    return await window.pluginAPI.call(PLUGIN_ID, method, (params ?? {}) as Record<string, unknown>);
  }, []);

  const loadStatus = async () => {
    const s = await call("ssh_status") as SshStatus;
    setSshStatus(s);
  };

  const loadRules = async () => {
    const r = await call("list_forward_rules") as ForwardRule[];
    setRules(r.filter(r => r.rule_type === "Manual"));
  };

  useEffect(() => {
    const init = async () => {
      try {
        const cfg = await call("get_config") as Record<string, unknown>;
        const ssh = cfg.ssh as Record<string, unknown> | undefined;
        if (ssh) {
          setForm({
            host: (ssh.host as string) || "",
            port: (ssh.port as number) || 22,
            username: (ssh.username as string) || "",
            password: (ssh.password as string) || "",
          });
        }
      } catch { /* ignore */ }
      const results = await Promise.allSettled([loadStatus(), loadRules()]);
      results.forEach((r, i) => { if (r.status === "rejected") console.warn(`init call ${i} failed:`, r.reason); });
    };
    init();
  }, []);

  const handleConnect = async () => {
    const hostInput = document.querySelector(".ssh-host-input") as HTMLInputElement;
    if (!form.host.trim()) {
      window.WorkTools.FieldError.show(hostInput, "SSH 主机地址不能为空");
      return;
    }
    try {
      await call("ssh_connect", form);
      window.WorkTools.toast.success("SSH 连接成功");
      loadStatus();
    } catch (e: unknown) { window.WorkTools.toast.error(`连接失败: ${e}`); }
  };

  const handleDisconnect = async () => {
    await call("ssh_disconnect");
    setSshStatus({ connected: false });
    window.WorkTools.toast.info("已断开");
  };

  const handleAdd = () => {
    const rule: ForwardRule = {
      id: window.crypto.randomUUID(),
      name: `rule-${Date.now()}`,
      local_host: "127.0.0.1",
      local_port: 0,
      remote_host: "",
      remote_port: 0,
      rule_type: "Manual" as const,
    };
    setEditing(rule);
    setIsNewRule(true);
  };

  const handleSave = async () => {
    if (!editing) return;
    try {
      if (isNewRule) {
        await call("add_forward_rule", editing);
      } else {
        await call("update_forward_rule", editing);
      }
      window.WorkTools.toast.success(isNewRule ? "规则已添加" : "已保存");
      setEditing(null);
      setIsNewRule(false);
      loadRules();
    } catch (e: unknown) { window.WorkTools.toast.error(`保存失败: ${e}`); }
  };

  const handleDelete = async (id: string) => {
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

  const clearHostError = () => {
    const hostInput = document.querySelector(".ssh-host-input") as HTMLInputElement;
    if (hostInput) window.WorkTools.FieldError.clear(hostInput);
  };

  return (
    <div>
      <div className="card">
        <div className="card-header">SSH 连接配置</div>
        <div className="form-row">
          <div className="form-group"><label>主机地址</label><input className="ssh-host-input" value={form.host} onChange={e => { setForm({...form, host: e.target.value}); clearHostError(); }} placeholder="10.73.x.x" /></div>
          <div className="form-group"><label>端口</label><input type="number" value={form.port} onChange={e => setForm({...form, port: +e.target.value})} /></div>
          <div className="form-group"><label>用户名</label><input value={form.username} onChange={e => setForm({...form, username: e.target.value})} /></div>
          <div className="form-group"><label>密码</label><input type="password" value={form.password} onChange={e => setForm({...form, password: e.target.value})} /></div>
          {sshStatus.connected
            ? <button className="btn btn-danger" onClick={handleDisconnect}>断开</button>
            : <button className="btn btn-primary" onClick={handleConnect}>连接</button>
          }
        </div>
        <div style={{marginTop:8}}>
          <span className={`status-dot ${sshStatus.connected ? "online" : "offline"}`}></span>
          {sshStatus.connected ? `已连接 → ${sshStatus.host}:${sshStatus.port}` : "未连接"}
        </div>
      </div>

      <div className="card">
        <div className="card-header" style={{display:"flex",justifyContent:"space-between",alignItems:"center"}}>
          <span>转发规则</span>
          <div style={{display:"flex",gap:8}}>
            <button className="btn btn-primary btn-sm" onClick={handleAdd}>+ 添加规则</button>
            <button className="btn btn-secondary btn-sm" onClick={handleImport}>导入</button>
            <button className="btn btn-secondary btn-sm" onClick={handleExport}>导出</button>
          </div>
        </div>
        <table>
          <thead><tr><th>名称</th><th>本地地址</th><th>本地端口</th><th>远程地址</th><th>远程端口</th><th>操作</th></tr></thead>
          <tbody>
            {rules.map(r => (
              <tr key={r.id}>
                <td>{r.name}</td>
                <td>{r.local_host}</td>
                <td>{r.local_port}</td>
                <td>{r.remote_host}</td>
                <td>{r.remote_port}</td>
                <td>
                  <button className="btn btn-secondary btn-sm" onClick={() => { setEditing(r); setIsNewRule(false); }} style={{marginRight:4}}>编辑</button>
                  <button className="btn btn-danger btn-sm" onClick={() => handleDelete(r.id)}>删除</button>
                </td>
              </tr>
            ))}
            {rules.length === 0 && <tr><td colSpan={6} style={{textAlign:"center",color:"var(--text-tertiary)",padding:20}}>暂无规则</td></tr>}
          </tbody>
        </table>
      </div>

      {editing && (
        <div className="modal-overlay" onClick={() => setEditing(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>编辑规则</h3>
            <div className="form-row">
              <div className="form-group"><label>名称</label><input value={editing.name} onChange={e => setEditing({...editing, name: e.target.value})} /></div>
              <div className="form-group"><label>本地地址</label><input value={editing.local_host} onChange={e => setEditing({...editing, local_host: e.target.value})} /></div>
              <div className="form-group"><label>本地端口</label><input type="number" value={editing.local_port} onChange={e => setEditing({...editing, local_port: +e.target.value})} /></div>
              <div className="form-group"><label>远程地址</label><input value={editing.remote_host} onChange={e => setEditing({...editing, remote_host: e.target.value})} /></div>
              <div className="form-group"><label>远程端口</label><input type="number" value={editing.remote_port} onChange={e => setEditing({...editing, remote_port: +e.target.value})} /></div>
            </div>
            <div className="modal-actions">
              <button className="btn btn-secondary" onClick={() => { setEditing(null); setIsNewRule(false); }}>取消</button>
              <button className="btn btn-primary" onClick={handleSave}>保存</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
