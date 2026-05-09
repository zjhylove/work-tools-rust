import { useState } from 'react';

interface Props {
  fields: Record<string, string>;
  selectedKey: string | null;
  onSetField: (field: string, value: string) => void;
  onDelField: (field: string) => void;
  searchQuery: string;
  multiSelect: Set<string>;
  onMultiToggle: (f: string) => void;
  onDeleteSelected: () => void;
}

export function HashViewer({ fields, selectedKey, onSetField, onDelField, searchQuery, multiSelect, onMultiToggle, onDeleteSelected }: Props) {
  const [newField, setNewField] = useState({ field: '', value: '' });
  const entries = Object.entries(fields).filter(([f, v]) =>
    !searchQuery || f.includes(searchQuery) || v.includes(searchQuery)
  );

  return (
    <div className="hash-editor">
      {multiSelect.size > 0 && (
        <div className="batch-bar">
          <span>已选 {multiSelect.size} 项</span>
          <button className="btn-danger" onClick={onDeleteSelected}>批量删除</button>
        </div>
      )}
      <table>
        <thead><tr><th /><th>Field</th><th>Value</th><th>操作</th></tr></thead>
        <tbody>
          {entries.map(([f, v]) => (
            <tr key={f}>
              <td><input type="checkbox" checked={multiSelect.has(f)}
                onChange={() => onMultiToggle(f)} /></td>
              <td><code>{f}</code></td>
              <td><code>{v}</code></td>
              <td><button onClick={() => onDelField(f)}>删除</button></td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="add-field">
        <input placeholder="field" value={newField.field} onChange={e => setNewField(p => ({ ...p, field: e.target.value }))} />
        <input placeholder="value" value={newField.value} onChange={e => setNewField(p => ({ ...p, value: e.target.value }))} />
        <button className="btn-primary" onClick={() => { onSetField(newField.field, newField.value); setNewField({ field: '', value: '' }); }}>
          添加</button>
      </div>
    </div>
  );
}
