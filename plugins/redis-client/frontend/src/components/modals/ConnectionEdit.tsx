import { useState, useEffect } from 'react';
import { ConnectionForm } from '../../types';
import { COLORS } from '../../types';
import { call } from '../../api';

declare global {
  interface Window {
    WorkTools: {
      toast: { success(m: string): void; error(m: string): void; info(m: string): void; warning(m: string): void };
    };
  }
  var WorkTools: Window['WorkTools'];
}

interface Props {
  connId: string | null;
  onClose: () => void;
  onSave: () => void;
}

const defaultForm: ConnectionForm = {
  name: '', color: null, host: '127.0.0.1', port: 6379, db: 0, password: '',
  ssh: null, cluster: null,
};

export function ConnectionEdit({ connId, onClose, onSave }: Props) {
  const [form, setForm] = useState<ConnectionForm>(defaultForm);
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (connId) {
      (async () => {
        try {
          const r = await call('list_connections');
          const conns = (r.connections as any[]) || [];
          const c = conns.find((x: any) => x.id === connId);
          if (c) {
            // 回显已保存的密码（包含 Redis 密码和 SSH 密码）
            let savedPassword = '';
            let savedSshPassword = '';
            let savedSshKeyPassphrase = '';
            try {
              const pw = await call('get_saved_password', { id: connId });
              savedPassword = (pw.password as string) || '';
              savedSshPassword = (pw.ssh_password as string) || '';
              savedSshKeyPassphrase = (pw.ssh_key_passphrase as string) || '';
            } catch { /* ignore */ }

            setForm({
              name: c.name || '',
              color: c.color || null,
              host: c.host || '127.0.0.1',
              port: c.port || 6379,
              db: c.db || 0,
              password: savedPassword,
              ssh: c.ssh ? {
                host: c.ssh.host || '',
                port: c.ssh.port || 22,
                username: c.ssh.username || '',
                authType: c.ssh.auth_type || 'password',
                password: savedSshPassword,
                keyPath: '',
                keyPassphrase: savedSshKeyPassphrase,
                timeoutSecs: c.ssh.timeout_secs || 10,
              } : null,
              cluster: c.cluster ? {
                seedNodes: c.cluster.seed_nodes || '',
              } : null,
            });
          }
        } catch { /* ignore */ }
      })();
    }
  }, [connId]);

  const handleTest = async () => {
    setTesting(true);
    try {
      await call('test_connection', {
        host: form.host, port: form.port, db: form.db, password: form.password,
        ssh: form.ssh ? {
          host: form.ssh.host, port: form.ssh.port, username: form.ssh.username,
          auth: form.ssh.authType === 'password'
            ? { type: 'password', password_obfuscated: form.ssh.password }
            : { type: 'key', key_path: form.ssh.keyPath, passphrase_obfuscated: form.ssh.keyPassphrase || null },
          timeout_secs: form.ssh.timeoutSecs,
        } : null,
        cluster: form.cluster ? { seed_nodes: form.cluster.seedNodes.split(',').map((s: string) => s.trim()) } : null,
      });
      WorkTools.toast.success('连接成功');
    } catch (e) { WorkTools.toast.error(`连接失败: ${e}`); }
    setTesting(false);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await call('save_connection', {
        id: connId || undefined,
        name: form.name, color: form.color, host: form.host, port: form.port, db: form.db,
        password: form.password,
        ssh: form.ssh ? {
          host: form.ssh.host, port: form.ssh.port, username: form.ssh.username,
          auth: form.ssh.authType === 'password'
            ? { type: 'password', password_obfuscated: form.ssh.password }
            : { type: 'key', key_path: form.ssh.keyPath, passphrase_obfuscated: form.ssh.keyPassphrase || null },
          timeout_secs: form.ssh.timeoutSecs,
        } : null,
        cluster: form.cluster ? { seed_nodes: form.cluster.seedNodes.split(',').map((s: string) => s.trim()) } : null,
      });
    } catch { /* ignore */ }
    setSaving(false);
    onSave();
  };

  return (
    <div className="modal-overlay">
      <div className="modal-content">
        <div className="modal-header">
          <h3>{connId ? '编辑连接' : '新建连接'}</h3>
          <button className="btn-secondary" onClick={onClose}>✕</button>
        </div>
        <div className="modal-body">
          <div className="form-group">
            <label>名称</label>
            <input value={form.name} onChange={e => setForm(p => ({ ...p, name: e.target.value }))} />
          </div>
          <div className="form-group">
            <label>颜色标记</label>
            <div className="color-options">
              {COLORS.map(c => (
                <span key={c} className={`color-dot ${form.color === c ? 'selected' : ''}`}
                  style={{ background: c }} onClick={() => setForm(p => ({ ...p, color: c }))} />
              ))}
            </div>
          </div>
          <div className="form-row">
            <div className="form-group flex-3"><label>Host</label>
              <input value={form.host} onChange={e => setForm(p => ({ ...p, host: e.target.value }))} /></div>
            <div className="form-group flex-1"><label>Port</label>
              <input type="number" value={form.port} onChange={e => setForm(p => ({ ...p, port: Number(e.target.value) }))} /></div>
          </div>
          <div className="form-group"><label>密码</label>
            <input type="password" value={form.password} onChange={e => setForm(p => ({ ...p, password: e.target.value }))} /></div>
          <div className="form-group"><label>DB</label>
            <input type="number" value={form.db} onChange={e => setForm(p => ({ ...p, db: Number(e.target.value) }))} /></div>

          <label className="checkbox-row">
            <input type="checkbox" checked={!!form.ssh} onChange={e => setForm(p => ({ ...p, ssh: e.target.checked ? { host: '', port: 22, username: '', authType: 'password', password: '', keyPath: '', keyPassphrase: '', timeoutSecs: 10 } : null }))} />
            通过 SSH 隧道连接
          </label>
          {form.ssh && (
            <div className="ssh-section">
              <div className="form-row">
                <div className="form-group flex-3"><label>SSH Host</label>
                  <input value={form.ssh.host} onChange={e => setForm(p => ({ ...p, ssh: { ...p.ssh!, host: e.target.value } }))} /></div>
                <div className="form-group flex-1"><label>Port</label>
                  <input type="number" value={form.ssh.port} onChange={e => setForm(p => ({ ...p, ssh: { ...p.ssh!, port: Number(e.target.value) } }))} /></div>
              </div>
              <div className="form-group"><label>用户名</label>
                <input value={form.ssh.username} onChange={e => setForm(p => ({ ...p, ssh: { ...p.ssh!, username: e.target.value } }))} /></div>
              <div className="form-group"><label>认证方式</label>
                <select value={form.ssh.authType} onChange={e => setForm(p => ({ ...p, ssh: { ...p.ssh!, authType: e.target.value as 'password' | 'key' } }))}>
                  <option value="password">密码</option>
                  <option value="key">私钥文件</option>
                </select>
              </div>
              {form.ssh.authType === 'password' ? (
                <div className="form-group"><label>SSH 密码</label>
                  <input type="password" value={form.ssh.password} onChange={e => setForm(p => ({ ...p, ssh: { ...p.ssh!, password: e.target.value } }))} /></div>
              ) : (
                <>
                  <div className="form-group"><label>私钥路径</label>
                    <input value={form.ssh.keyPath} onChange={e => setForm(p => ({ ...p, ssh: { ...p.ssh!, keyPath: e.target.value } }))} /></div>
                  <div className="form-group"><label>私钥密码（可选）</label>
                    <input type="password" value={form.ssh.keyPassphrase} onChange={e => setForm(p => ({ ...p, ssh: { ...p.ssh!, keyPassphrase: e.target.value } }))} /></div>
                </>
              )}
            </div>
          )}

          <label className="checkbox-row">
            <input type="checkbox" checked={!!form.cluster} onChange={e => setForm(p => ({ ...p, cluster: e.target.checked ? { seedNodes: '' } : null }))} />
            Cluster 模式
          </label>
          {form.cluster && (
            <div className="form-group"><label>种子节点（逗号分隔 host:port）</label>
              <input value={form.cluster.seedNodes} onChange={e => setForm(p => ({ ...p, cluster: { seedNodes: e.target.value } }))}
                placeholder="host1:7000,host2:7001" />
            </div>
          )}
        </div>
        <div className="modal-footer">
          <button className="btn-secondary" onClick={handleTest} disabled={testing}>{testing ? '测试中…' : '测试连接'}</button>
          <button className="btn-secondary" onClick={onClose}>取消</button>
          <button className="btn-accent" onClick={handleSave} disabled={saving}>{saving ? '保存中…' : '保存'}</button>
        </div>
      </div>
    </div>
  );
}
