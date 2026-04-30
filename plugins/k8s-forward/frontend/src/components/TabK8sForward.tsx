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
  const [editingForward, setEditingForward] = useState<{ rule_id: string; domain: string; local_port: number; pod_name: string; remote_host: string; remote_port: number; local_host: string } | null>(null);

  const call = useCallback(async (method: string, params?: unknown) => {
    return await window.pluginAPI.call(PLUGIN_ID, method, (params ?? {}) as Record<string, unknown>);
  }, []);

  const showToast = (msg: string, isErr = false) => {
    setToast(isErr ? `❌ ${msg}` : `✅ ${msg}`);
    setTimeout(() => setToast(null), 3000);
  };

  const loadStatus = async () => { setKstatus(await call("kuboard_status") as KuboardStatus); };
  const loadForwards = async () => { setForwards(await call("list_k8s_forwards") as K8sForwardInfo); };
  const validateForwards = async () => {
    try {
      const result = await call("validate_k8s_forwards") as { removed: number };
      if (result.removed > 0) showToast(`已清理 ${result.removed} 个无效转发`);
    } catch { /* Kuboard 未登录时忽略 */ }
  };

  useEffect(() => {
    const init = async () => {
      try {
        const cfg = await call("get_config") as Record<string, unknown>;
        const kb = cfg.kuboard as Record<string, unknown> | undefined;
        if (kb) {
          setLoginForm({
            url: (kb.url as string) || "http://10.73.64.28:8087",
            username: (kb.username as string) || "",
            password: (kb.password as string) || "",
          });
        }
      } catch { /* ignore */ }
      const results = await Promise.allSettled([loadStatus(), loadForwards()]);
      results.forEach((r, i) => { if (r.status === "rejected") console.warn(`init call ${i} failed:`, r.reason); });
    };
    init();
  }, []);

  const handleLogin = async () => {
    try {
      const r = await call("kuboard_login", loginForm) as LoginResult;
      if (r.mfa_required) { setMfaRequired(true); showToast("请输入 MFA 验证码"); }
      else if (r.success) {
        showToast("登录成功");
        await Promise.allSettled([loadStatus(), loadClusters()]);
        await validateForwards();
        loadForwards();
      }
      else { showToast(r.message || "登录失败", true); }
    } catch (e: unknown) { showToast(`登录失败: ${e}`, true); }
  };

  const handleMfa = async () => {
    try {
      await call("kuboard_mfa", { passcode });
      setMfaRequired(false); setPasscode("");
      showToast("登录成功");
      await Promise.allSettled([loadStatus(), loadClusters()]);
      await validateForwards();
      loadForwards();
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

  const handleUpdateForward = async () => {
    if (!editingForward) return;
    try {
      await call("update_proxy_mapping", { rule_id: editingForward.rule_id, domain: editingForward.domain });
      await call("update_forward_rule", {
        id: editingForward.rule_id,
        name: editingForward.pod_name,
        local_host: editingForward.local_host,
        local_port: editingForward.local_port,
        remote_host: editingForward.remote_host,
        remote_port: editingForward.remote_port,
        rule_type: "K8s",
        pod_name: editingForward.pod_name,
        container_name: "",
        cluster: "",
        namespace: "",
      });
      showToast("已更新");
      setEditingForward(null);
      loadForwards();
    } catch (e: unknown) { showToast(`更新失败: ${e}`, true); }
  };

  const filteredPods = pods.filter(p => p.status === "Running" && p.name.toLowerCase().includes(search.toLowerCase()));

  const isForwarded = (podName: string, containerName: string, port: number) =>
    forwards.rules.some(r => r.pod_name === podName && r.container_name === containerName && r.remote_port === port);

  const getForwardMapping = (podName: string, containerName: string, port: number) => {
    const rule = forwards.rules.find(r => r.pod_name === podName && r.container_name === containerName && r.remote_port === port);
    if (!rule) return null;
    const mapping = forwards.mappings.find(m => m.rule_id === rule.id && m.editable);
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
              <button className="btn btn-secondary btn-sm" onClick={() => loadPods(selCluster, selNs)}>刷新 Pod</button>
            </div>
          </div>

          <div className="card">
            <div className="card-header" style={{display:"flex",justifyContent:"space-between"}}>
              <span>Pod 列表 ({filteredPods.length})</span>
              <input placeholder="搜索 Pod..." value={search} onChange={e => setSearch(e.target.value)} style={{padding:"4px 8px",border:"1px solid #e5e5e5",borderRadius:4,background:"#fafafa",color:"#1a1a1a",fontSize:12,width:200}} />
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
                <thead><tr><th>Pod名称</th><th>Pod地址</th><th>本地端口</th><th>目标</th><th>操作</th></tr></thead>
                <tbody>
                  {forwards.rules.map(r => {
                    const m = forwards.mappings.find(m => m.rule_id === r.id && m.editable);
                    return (
                      <tr key={r.id}>
                        <td>{r.pod_name || "-"}</td>
                        <td>{m?.domain || `${r.remote_host}:${r.remote_port}`}</td>
                        <td>{r.local_port}</td>
                        <td>{r.remote_host}:{r.remote_port}</td>
                        <td>
                          <button className="btn btn-secondary btn-sm" style={{marginRight:4}} onClick={() => setEditingForward({ rule_id: r.id, domain: m?.domain || `${r.remote_host}:${r.remote_port}`, local_port: r.local_port, pod_name: r.pod_name || "", remote_host: r.remote_host, remote_port: r.remote_port, local_host: r.local_host })}>编辑</button>
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

      {editingForward && (
        <div className="modal-overlay" onClick={() => setEditingForward(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>编辑转发</h3>
            <div className="form-row">
              <div className="form-group"><label>Pod地址</label><input value={editingForward.domain} onChange={e => setEditingForward({...editingForward, domain: e.target.value})} /></div>
              <div className="form-group"><label>本地端口</label><input type="number" value={editingForward.local_port} onChange={e => setEditingForward({...editingForward, local_port: +e.target.value})} /></div>
            </div>
            <div className="modal-actions">
              <button className="btn btn-secondary" onClick={() => setEditingForward(null)}>取消</button>
              <button className="btn btn-primary" onClick={handleUpdateForward}>保存</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
