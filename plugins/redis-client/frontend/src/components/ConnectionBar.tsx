import { SavedConnection } from '../types';

interface Props {
  savedConns: SavedConnection[];
  currentId: string | null;
  onConnect: (id: string) => void;
  onDisconnect: () => void;
  onManage: () => void;
}

export function ConnectionBar({ savedConns, currentId, onConnect, onDisconnect, onManage }: Props) {
  return (
    <div className="connection-bar">
      <div className="connection-selector">
        <span className="status-dot" />
        <select value={currentId || ''} onChange={async e => {
          if (e.target.value) {
            onDisconnect();
            onConnect(e.target.value);
          }
        }}>
          <option value="" disabled>选择连接...</option>
          {savedConns.map(c => (
            <option key={c.id} value={c.id}>{c.name} ({c.host}:{c.port})</option>
          ))}
        </select>
      </div>
      <div className="connection-actions">
        <button onClick={onManage} title="管理连接">⚙</button>
        <button onClick={onDisconnect} title="断开连接">✕</button>
      </div>
    </div>
  );
}
