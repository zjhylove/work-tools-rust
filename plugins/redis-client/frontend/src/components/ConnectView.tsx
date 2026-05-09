import { useState } from 'react';
import { SavedConnection } from '../types';

interface Props {
  savedConns: SavedConnection[];
  onConnect: (id: string, password?: string) => void;
  onManage: () => void;
  onRefresh: () => void;
}

const COLORS = ['#ef4444', '#f59e0b', '#10b981', '#3b82f6', '#8b5cf6', '#ec4899', '#06b6d4', '#6b7280'];

export function ConnectView({ savedConns, onConnect, onManage, onRefresh }: Props) {
  const [passwordMap, setPasswordMap] = useState<Record<string, string>>({});

  return (
    <div className="connect-view">
      <div className="connect-header">
        <h3>Redis 连接</h3>
        <button onClick={onManage}>管理连接</button>
      </div>
      <div className="saved-connections">
        {savedConns.map((c, i) => (
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
    </div>
  );
}
