import { useState } from 'react';
import { SavedConnection } from '../types';
import { COLORS, call } from '../api';

interface Props {
  savedConns: SavedConnection[];
  onConnect: (id: string, password?: string) => void;
  onQuickConnect: (host: string, port: number, db: number, password: string) => void;
  onManage: () => void;
}

export function ConnectView({ savedConns, onConnect, onQuickConnect, onManage }: Props) {
  const [mode, setMode] = useState<'saved' | 'quick'>('saved');
  const [passwordMap, setPasswordMap] = useState<Record<string, string>>({});
  const [quick, setQuick] = useState({ host: '127.0.0.1', port: 6379, db: 0, password: '' });
  const [connecting, setConnecting] = useState(false);

  const handleQuickConnect = async () => {
    setConnecting(true);
    try {
      await call('connect', { host: quick.host, port: quick.port, db: quick.db, password: quick.password });
      onQuickConnect(quick.host, quick.port, quick.db, quick.password);
    } catch (e) {
      alert(`连接失败: ${e}`);
    }
    setConnecting(false);
  };

  return (
    <div className="connect-view">
      <div className="connect-header">
        <h3>Redis 连接</h3>
        <button onClick={onManage}>管理连接</button>
      </div>

      <div className="connect-tabs">
        <button className={mode === 'quick' ? 'active' : ''} onClick={() => setMode('quick')}>快速连接</button>
        <button className={mode === 'saved' ? 'active' : ''} onClick={() => setMode('saved')}>已保存</button>
      </div>

      {mode === 'quick' ? (
        <div className="quick-connect-form">
          <div className="form-group">
            <label>Host</label>
            <input value={quick.host} onChange={e => setQuick(p => ({ ...p, host: e.target.value }))}
              onKeyDown={e => e.key === 'Enter' && handleQuickConnect()} />
          </div>
          <div className="form-row">
            <div className="form-group flex-3">
              <label>Port</label>
              <input type="number" value={quick.port} onChange={e => setQuick(p => ({ ...p, port: Number(e.target.value) }))} />
            </div>
            <div className="form-group flex-1">
              <label>DB</label>
              <input type="number" value={quick.db} onChange={e => setQuick(p => ({ ...p, db: Number(e.target.value) }))} />
            </div>
          </div>
          <div className="form-group">
            <label>密码</label>
            <input type="password" value={quick.password} onChange={e => setQuick(p => ({ ...p, password: e.target.value }))}
              placeholder="可选" onKeyDown={e => e.key === 'Enter' && handleQuickConnect()} />
          </div>
          <button className="btn-primary" onClick={handleQuickConnect} disabled={connecting}>
            {connecting ? '连接中…' : '连接'}
          </button>
        </div>
      ) : (
        <div className="saved-connections">
          {savedConns.length === 0 ? (
            <div className="list-status">暂无已保存的连接</div>
          ) : savedConns.map((c, i) => (
            <div key={c.id} className="saved-conn-item">
              <div className="saved-conn-main" onClick={() => onConnect(c.id, passwordMap[c.id])}>
                <div className="conn-left">
                  <span className="conn-color-dot" style={{ background: c.color || COLORS[i % COLORS.length] }} />
                  <div>
                    <div className="conn-name">{c.name}</div>
                    <div className="conn-info">{c.host}:{c.port} db{c.db}</div>
                  </div>
                </div>
                <div className="conn-tags">
                  {c.has_ssh && <span className="conn-badge">SSH</span>}
                  {c.has_cluster && <span className="conn-badge">Cluster</span>}
                </div>
              </div>
              {c.has_password && (
                <input type="password" placeholder="密码" className="conn-password"
                  onChange={e => setPasswordMap(p => ({ ...p, [c.id]: e.target.value }))}
                  onClick={e => e.stopPropagation()} />
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
