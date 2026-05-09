import { useState } from 'react';
import { SavedConnection } from '../types';
import { ConnectionEdit } from './modals/ConnectionEdit';
import { DeleteConfirm } from './modals/DeleteConfirm';

interface Props {
  savedConns: SavedConnection[];
  onBack: () => void;
  onSave: () => void;
  onDelete: (id: string) => void;
  editId: string | null;
  onEditStart: (id: string | null) => void;
}

const COLORS = ['#ef4444', '#f59e0b', '#10b981', '#3b82f6', '#8b5cf6', '#ec4899', '#06b6d4', '#6b7280'];

export function ConnectionManager({ savedConns, onBack, onSave, onDelete, editId, onEditStart }: Props) {
  const [deleteId, setDeleteId] = useState<string | null>(null);

  return (
    <div className="connection-manager">
      <div className="manager-header">
        <button onClick={onBack}>← 返回</button>
        <h3>连接管理</h3>
        <button className="btn-primary" onClick={() => onEditStart(null)}>+ 新建</button>
      </div>
      <div className="manager-list">
        {savedConns.map((c, i) => (
          <div key={c.id} className="conn-card">
            <div className="conn-card-color" style={{ background: c.color || COLORS[i % COLORS.length] }} />
            <div className="conn-card-info">
              <div className="conn-card-name">{c.name}</div>
              <div className="conn-card-detail">{c.host}:{c.port} db{c.db}</div>
              {c.has_ssh && <span className="conn-badge">SSH</span>}
              {c.has_cluster && <span className="conn-badge">Cluster</span>}
            </div>
            <div className="conn-card-actions">
              <button onClick={() => onEditStart(c.id)}>编辑</button>
              <button className="btn-danger-text" onClick={() => setDeleteId(c.id)}>删除</button>
            </div>
          </div>
        ))}
      </div>

      {editId !== null && (
        <ConnectionEdit
          connId={editId}
          onClose={() => onEditStart(null)}
          onSave={() => { onSave(); onEditStart(null); }}
        />
      )}

      {deleteId && (
        <DeleteConfirm
          message="确定删除此连接？"
          onConfirm={() => { onDelete(deleteId); setDeleteId(null); }}
          onCancel={() => setDeleteId(null)}
        />
      )}
    </div>
  );
}
