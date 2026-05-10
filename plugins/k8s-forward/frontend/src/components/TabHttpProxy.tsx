import { useState, useEffect, useCallback } from "react";
import type { ProxyStatus, ProxyMapping } from "../types";

declare global {
  interface Window { WorkTools: { toast: { success(m:string):void; error(m:string):void; info(m:string):void; warning(m:string):void }; FieldError: { show(el:HTMLElement, m:string):void; clear(el:HTMLElement):void; clearAll(f:HTMLElement):void } } }
}

const PLUGIN_ID = "k8s-forward";

interface MappingGroup {
  name: string;
  addr: string;
  target: string;
  rule_id: string;
}

export default function TabHttpProxy() {
  const [status, setStatus] = useState<ProxyStatus>({ running: false, port: 80, mapping_count: 0 });
  const [mappings, setMappings] = useState<ProxyMapping[]>([]);
  const [port, setPort] = useState(80);
  const [editing, setEditing] = useState<{ rule_id: string; domain: string; target: string } | null>(null);

  const call = useCallback(async (method: string, params?: unknown) => {
    return await window.pluginAPI.call(PLUGIN_ID, method, (params ?? {}) as Record<string, unknown>);
  }, []);

  const refresh = async () => {
    const s = await call("proxy_status") as ProxyStatus;
    setStatus(s);
    if (s.running) {
      const m = await call("list_proxy_mappings") as ProxyMapping[];
      setMappings(m);
    }
  };

  useEffect(() => { try { refresh(); } catch { /* ignore */ } }, []);

  const handleStart = async () => {
    const portInput = document.querySelector(".proxy-port-input") as HTMLInputElement;
    if (!port || port < 1 || port > 65535) {
      window.WorkTools.FieldError.show(portInput, "请输入有效的端口号 (1-65535)");
      return;
    }
    try {
      await call("proxy_start", { port });
      window.WorkTools.toast.success(`代理已启动: 127.0.0.1:${port}`);
      refresh();
    } catch (e: unknown) { window.WorkTools.toast.error(`启动失败: ${e}`); }
  };

  const handleStop = async () => {
    await call("proxy_stop");
    window.WorkTools.toast.info("代理已停止");
    refresh();
  };

  const handleUpdate = async () => {
    if (!editing) return;
    try {
      await call("update_proxy_mapping", { rule_id: editing.rule_id, domain: editing.domain });
      window.WorkTools.toast.success("Pod地址已更新");
      setEditing(null);
      refresh();
    } catch (e: unknown) { window.WorkTools.toast.error(`更新失败: ${e}`); }
  };

  const clearPortError = () => {
    const portInput = document.querySelector(".proxy-port-input") as HTMLInputElement;
    if (portInput) window.WorkTools.FieldError.clear(portInput);
  };

  // 按 rule_id 分组：每个转发有一条 pod 名（别名）和一条 pod 地址
  const groups = mappings.reduce<Record<string, MappingGroup>>((acc, m) => {
    if (!acc[m.rule_id]) {
      acc[m.rule_id] = { name: "", addr: "", target: m.target, rule_id: m.rule_id };
    }
    if (m.editable) {
      acc[m.rule_id].addr = m.domain;
    } else {
      acc[m.rule_id].name = m.domain;
    }
    return acc;
  }, {});

  return (
    <div>
      <div className="card">
        <div className="card-header">HTTP 代理服务器</div>
        <div className="form-row">
          <div className="form-group">
            <label>代理端口</label>
            <input className="proxy-port-input" type="number" value={port} onChange={e => { setPort(+e.target.value); clearPortError(); }} disabled={status.running} style={{width:80}} />
          </div>
          {status.running
            ? <button className="btn btn-danger" onClick={handleStop}>停止</button>
            : <button className="btn btn-primary" onClick={handleStart}>启动</button>
          }
        </div>
        <div style={{marginTop:8}}>
          <span className={`status-dot ${status.running ? "online" : "offline"}`}></span>
          {status.running ? `运行中 → 127.0.0.1:${status.port} (${status.mapping_count} 条映射)` : "已停止"}
        </div>
      </div>

      {status.running && (
        <div className="card">
          <div className="card-header">代理映射表</div>
          <table>
            <thead><tr><th>Pod名称</th><th>Pod地址</th><th>目标地址</th><th>操作</th></tr></thead>
            <tbody>
              {Object.values(groups).map(g => (
                <tr key={g.rule_id}>
                  <td><code>{g.name}</code></td>
                  <td><code>{g.addr}</code></td>
                  <td>{g.target}</td>
                  <td>
                    <button className="btn btn-secondary btn-sm" onClick={() => setEditing({ rule_id: g.rule_id, domain: g.addr, target: g.target })}>编辑</button>
                  </td>
                </tr>
              ))}
              {Object.keys(groups).length === 0 && <tr><td colSpan={4} style={{textAlign:"center",color:"var(--text-tertiary)",padding:20}}>暂无映射</td></tr>}
            </tbody>
          </table>
        </div>
      )}

      {editing && (
        <div className="modal-overlay" onClick={() => setEditing(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>编辑 Pod 地址</h3>
            <div className="form-group"><label>Pod地址</label><input value={editing.domain} onChange={e => setEditing({...editing, domain: e.target.value})} /></div>
            <div style={{marginTop:8,fontSize:11,color:"var(--text-tertiary)"}}>目标: {editing.target}</div>
            <div className="modal-actions">
              <button className="btn btn-secondary" onClick={() => setEditing(null)}>取消</button>
              <button className="btn btn-primary" onClick={handleUpdate}>保存</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
