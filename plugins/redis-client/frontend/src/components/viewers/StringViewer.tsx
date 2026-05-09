import { useState, useEffect } from 'react';
import { isJson, formatJson, compressJson } from '../../utils/json';

interface Props {
  value: { value: string };
  selectedKey: string | null;
  onSave: (value: string) => void;
}

export function StringViewer({ value, selectedKey, onSave }: Props) {
  const [editing, setEditing] = useState('');
  const [formatted, setFormatted] = useState(false);

  useEffect(() => {
    setEditing(value.value);
    setFormatted(isJson(value.value));
  }, [value.value]);

  return (
    <div className="value-editor">
      <div className="viewer-actions">
        {isJson(value.value) && (
          <button onClick={() => {
            setEditing(prev => formatted ? compressJson(prev) : formatJson(prev));
            setFormatted(!formatted);
          }}>{formatted ? '压缩' : '格式化'}</button>
        )}
      </div>
      <textarea value={editing} onChange={e => setEditing(e.target.value)} rows={14} />
      <button className="btn-primary" onClick={() => onSave(editing)}>保存</button>
    </div>
  );
}
