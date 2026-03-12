import type { ValidationError } from '../utils/jsonUtils';

interface JsonEditorProps {
  value: string;
  onChange: (value: string) => void;
  error: ValidationError | null;
}

export default function JsonEditor({ value, onChange }: JsonEditorProps) {
  return (
    <div className="json-editor-panel">
      <textarea
        className="json-editor"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="在此输入或粘贴 JSON..."
        spellCheck={false}
      />
    </div>
  );
}
