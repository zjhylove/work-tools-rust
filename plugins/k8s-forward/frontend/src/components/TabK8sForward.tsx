import { useState, useEffect, useCallback } from "react";
import type { KuboardStatus, PodInfo, K8sForwardInfo, ForwardRule, ProxyMapping, LoginResult } from "../types";

const PLUGIN_ID = "k8s-forward";

export default function TabK8sForward() {
  const [kstatus, setKstatus] = useState<KuboardStatus>({ logged_in: false });
  const [mfaRequired, setMfaRequired] = useState(false);
  const [loginForm, setLoginForm] = useState({ url: "http://10.73.64.28:8087", username: "", password: "" });
  const [passcode, setPasscode] = useState("");
  const [clusters, setClusters] = useState<string[]>([]);
  const [selCluster, setSelCluster] = useState("");
  const [namespaces, setNamespaces] = useState<string[]>([]);
  const [selNs, setSelNs] = useState("");
  const [pods, setPods] = useState<PodInfo[]>([]);
  const [search, setSearch] = useState("");
  const [forwards, setForwards] = useState<K8sForwardInfo>({ rules: [], mappings: [] });
  const [toast, setToast] = useState<string | null>(null);
  const [editingDomain, setEditingDomain] = useState<{ rule_id: string; domain: string } | null>(null);

  const call = useCallback(async (method: string, params?: unknown) => {
    return await window.pluginAPI.call(PLUGIN_ID, method, (params ?? {}) as Record<string, unknown>);
  }, []);

  const showToast = (msg: string, isErr = false) => {
    setToast(isErr ? `❌ ${msg}` : `✅ ${msg}`);
    setTimeout(() => setToast(null), 3000);
  };

  const loadStatus = async () => { setKstatus(await call("kuboard_status") as KuboardStatus); };
  const loadForwards = async () => { setForwards(await call("list_k8s_forwards") as K8sForwardInfo); };

  useEffect(() => { loadStatus(); loadForwards(); }, []);

  const handleLogin = async () => {
    try {
      const r = await call("kuboard_login", loginForm) as LoginResult;
      if (r.mfa_required) { setMfaRequired(true); showToast("请输入 MFA 验证码"); }
      else if (r.success) { showToast("登录成功"); loadStatus(); }
      else { showToast(r.message || "登录失败", true); }
    } catch (e: unknown) { showToast(`登录失败: ${e}`, true); }
  };

  const handleMfa = async () => {
    try {
      await call("kuboard_mfa", { passcode });
      setMfaRequired(false); setPasscode("");
      showToast("登录成功");
      loadStatus();
    } catch (e: unknown) { showToast(`MFA 验证失败: ${e}`, true); }
  };

  const handleLogout = async () => {
    await call("kuboard_logout");
    setKstatus({ logged_in: false });
    setClusters([]); setNamespaces([]); setPods([]);
  };

  const loadClusters = async () => {
    try {
      const c = await call("list_clusters") as string[];
      setClusters(c);
      if (c.length > 0) { setSelCluster(c[0]); loadNamespaces(c[0]); }
    } catch (e: unknown) { showToast(`获取集群失败: ${e}`, true); }
  };

  const loadNamespaces = async (cluster: string) => {
    try {
      const ns = await call("list_namespaces", { cluster }) as string[];
      setNamespaces(ns);
      if (ns.length > 0) { setSelNs(ns[0]); loadPods(cluster, ns[0]); }
    } catch (e: unknown) { showToast(`获取命名空间失败: ${e}`, true); }
  };

  const loadPods = async (cluster: string, ns: string) => {
    try {
      const p = await call("list_pods", { cluster, namespace: ns }) as PodInfo[];
      setPods(p);
    } catch (e: unknown) { showToast(`获取 Pod 失败: ${e}`, true); }
  };

  const handleForward = async (podName: string, containerName: string, containerPort: number) => {
    try {
      await call("forward_pod", { cluster: selCluster, namespace: selNs, pod_name: podName, container_name: containerName, container_port: containerPort });
      showToast(`已转发 ${podName}/${containerName}:${containerPort}`);
      loadForwards();
    } catch (e: unknown) { showToast(`转发失败: ${e}`, true); }
  };

  const handleUnforward = async (ruleId: string) => {
    try {
      await call("unforward_pod", { rule_id: ruleId });
      showToast("已取消转发");
      loadForwards();
    } catch (e: unknown) { showToast(`取消失败: ${e}`, true); }
  };

  const handleUpdateDomain = async () => {
    if (!editingDomain) return;
    try {
      await call("update_proxy_mapping", editingDomain);
      showToast("域名已更新");
      setEditingDomain(null);
      loadForwards();
    } catch (e: unknown) { showToast(`更新失败: ${e}`, true); }
  };

  const filteredPods = pods.filter(p => p.name.toLowerCase().includes(search.toLowerCase()));

  const isForwarded = (podName: string, containerName: string, port: number) =>
    forwards.rules.some(r => r.pod_name === podName && r.container_name === containerName && r.remote_port === port);

  const getForwardMapping = (podName: string, containerName: string, port: number) => {
    const rule = forwards.rules.find(r => r.pod_name === podName && r.container_name === containerName && r.remote_port === port);
    if (!rule) return null;
    const mapping = forwards.mappings.find(m => m.rule_id === rule.id);
    return { rule, mapping };
  };

  return (
    <div>
      {toast && <div className={`toast ${toast.startsWith("❌") ? "toast-error" : "toast-success"}`}>{toast}</div>}

      <div className="card">
        <div className="card-header">Kuboard 连接</div>
        <div className="form-row">
          <div className="form-group"><label>Kuboard 地址</label><input value={loginForm.url} onChange={e => setLoginForm({...loginForm, url: e.target.value})} style={{minWidth:220}} /></div>
          <div className="form-group"><label>用户名</label><input value={loginForm.username} onChange={e => setLoginForm({...loginForm, username: e.target.value})} /></div>
          <div className="form-group"><label>密码</label><input type="password" value={loginForm.password} onChange={e => setLoginForm({...loginForm, password: e.target.value})} /></div>
          {kstatus.logged_in
            ? <button className="btn btn-danger" onClick={handleLogout}>登出</button>
            : <button className="btn btn-primary" onClick={handleLogin}>登录</button>
          }
        </div>
        <div style={{marginTop:8}}>
          <span className={`status-dot ${kstatus.logged_in ? "online" : "offline"}`}></span>
          {kstatus.logged_in ? `已登录 → ${kstatus.username}@${kstatus.url}` : "未登录"}
        </div>
      </div>

      {mfaRequired && (
        <div className="card" style={{borderColor:"#ffa502"}}>
          <div className="card-header">双因子认证</div>
          <div className="form-row">
            <div className="form-group"><label>验证码</label><input value={passcode} onChange={e => setPasscode(e.target.value)} placeholder="6位验证码" maxLength={6} /></div>
            <button className="btn btn-primary" onClick={handleMfa}>验证</button>
          </div>
        </div>
      )}

      {kstatus.logged_in && (
        <>
          <div className="card">
            <div className="card-header">集群 & 命名空间</div>
            <div className="form-row">
              <div className="form-group">
                <label>集群</label>
                <select value={selCluster} onChange={e => { setSelCluster(e.target.value); loadNamespaces(e.target.value); }}>
                  {clusters.length === 0 && <option>-- 点击加载 --</option>}
                  {clusters.map(c => <option key={c} value={c}>{c}</option>)}
                </select>
              </div>
              <div className="form-group">
                <label>命名空间</label>
                <select value={selNs} onChange={e => { setSelNs(e.target.value); loadPods(selCluster, e.target.value); }}>
                  {namespaces.length === 0 && <option>-- 选择集群后加载 --</option>}
                  {namespaces.map(n => <option key={n} value={n}>{n}</option>)}
                </select>
              </div>
              <button className="btn btn-primary btn-sm" onClick={loadClusters}>加载集群</button>
              <button className="btn btn-default btn-sm" onClick={() => loadPods(selCluster, selNs)}>刷新 Pod</button>
            </div>
          </div>

          <div className="card">
            <div className="card-header" style={{display:"flex",justifyContent:"space-between"}}>
              <span>Pod 列表 ({filteredPods.length})</span>
              <input placeholder="搜索 Pod..." value={search} onChange={e => setSearch(e.target.value)} style={{padding:"4px 8px",border:"1px solid #3a3a5a",borderRadius:4,background:"#12122a",color:"#e0e0e0",fontSize:12,width:200}} />
            </div>
            <div style={{maxHeight:400,overflow:"auto"}}>
              <table>
                <thead><tr><th>Pod</th><th>IP</th><th>容器</th><th>端口</th><th>状态</th><th>操作</th></tr></thead>
                <tbody>
                  {filteredPods.map(p => (
                    p.containers.map((c, ci) => (
                      c.ports.map((pt, pti) => {
                        const fwd = isForwarded(p.name, c.name, pt.container_port);
                        const fm = getForwardMapping(p.name, c.name, pt.container_port);
                        return (
                          <tr key={`${p.name}-${ci}-${pti}`}>
                            {pti === 0 && ci === 0 && <td rowSpan={p.containers.reduce((a,c) => a + Math.max(c.ports.length, 1), 0)}>{p.name}</td>}
                            {pti === 0 && ci === 0 && <td rowSpan={p.containers.reduce((a,c) => a + Math.max(c.ports.length, 1), 0)}>{p.ip}</td>}
                            {pti === 0 && <td rowSpan={Math.max(c.ports.length, 1)}>{c.name}</td>}
                            <td>{pt.container_port}/{pt.protocol}</td>
                            <td><span className={`badge ${p.status === "Running" ? "badge-success" : "badge-warning"}`}>{p.status}</span></td>
                            <td>
                              {fwd
                                ? <button className="btn btn-danger btn-sm" onClick={() => handleUnforward(fm?.rule.id || "")}>取消</button>
                                : <button className="btn btn-primary btn-sm" onClick={() => handleForward(p.name, c.name, pt.container_port)}>转发</button>
                              }
                            </td>
                          </tr>
                        );
                      })
                    ))
                  ))}
                  {filteredPods.length === 0 && <tr><td colSpan={6} style={{textAlign:"center",color:"#666",padding:20}}>无 Pod</td></tr>}
                </tbody>
              </table>
            </div>
          </div>

          {forwards.rules.length > 0 && (
            <div className="card">
              <div className="card-header">已转发列表</div>
              <table>
                <thead><tr><th>域名</th><th>本地端口</th><th>目标</th><th>操作</th></tr></thead>
                <tbody>
                  {forwards.rules.map(r => {
                    const m = forwards.mappings.find(m => m.rule_id === r.id);
                    return (
                      <tr key={r.id}>
                        <td>{m?.domain || "-"}</td>
                        <td>{r.local_port}</td>
                        <td>{r.remote_host}:{r.remote_port}</td>
                        <td>
                          <button className="btn btn-default btn-sm" style={{marginRight:4}} onClick={() => navigator.clipboard.writeText(m?.domain || "")}>复制域名</button>
                          <button className="btn btn-default btn-sm" style={{marginRight:4}} onClick={() => setEditingDomain({ rule_id: r.id, domain: m?.domain || "" })}>编辑</button>
                          <button className="btn btn-danger btn-sm" onClick={() => handleUnforward(r.id)}>取消</button>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}

      {editingDomain && (
        <div className="modal-overlay" onClick={() => setEditingDomain(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>编辑域名</h3>
            <div className="form-group"><label>域名</label><input value={editingDomain.domain} onChange={e => setEditingDomain({...editingDomain, domain: e.target.value})} style={{width:"100%"}} /></div>
            <div className="modal-actions">
              <button className="btn btn-default" onClick={() => setEditingDomain(null)}>取消</button>
              <button className="btn btn-primary" onClick={handleUpdateDomain}>保存</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
