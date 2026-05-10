import { useState } from 'react';
import { SavedConnection, COLORS } from '../types';
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

export function ConnectionManager({ savedConns, onBack, onSave, onDelete, editId, onEditStart }: Props) {
  const [deleteId, setDeleteId] = useState<string | null>(null);

  return (
    <div className="connection-manager">
      <div className="manager-header">
        <button className="btn-back" onClick={onBack}>&#8592; 返回</button>
        <h3>连接管理</h3>
        <button className="btn-accent" onClick={() => onEditStart('')}>+ 新建</button>
      </div>

      {savedConns.length === 0 ? (
        <div className="manager-empty">
          <div className="empty-icon">&#9997;</div>
          <div className="empty-text">还没有保存的连接</div>
          <button className="btn-accent" onClick={() => onEditStart('')}>创建第一个连接</button>
        </div>
      ) : (
        <div className="manager-grid">
          {savedConns.map((c, i) => {
            const color = c.color || COLORS[i % COLORS.length];
            return (
              <div key={c.id} className="conn-card" style={{ borderLeftColor: color }}>
                <div className="conn-card-header">
                  <span className="conn-card-name">{c.name}</span>
                  <span className="conn-card-dot" style={{ background: color }} />
                </div>
                <div className="conn-card-body">
                  <span className="conn-card-addr">{c.host}:{c.port}</span>
                  <span className="conn-card-db">db{c.db}</span>
                </div>
                {(c.has_ssh || c.has_cluster) && (
                  <div className="conn-card-tags">
                    {c.has_ssh && <span className="conn-tag">SSH</span>}
                    {c.has_cluster && <span className="conn-tag">Cluster</span>}
                  </div>
                )}
                <div className="conn-card-actions">
                  <button className="btn-icon" title="编辑" onClick={() => onEditStart(c.id)}>&#9998;</button>
                  <button className="btn-icon btn-icon-danger" title="删除" onClick={() => setDeleteId(c.id)}>&#10005;</button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {editId !== null && (
        <ConnectionEdit
          connId={editId || null}
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
